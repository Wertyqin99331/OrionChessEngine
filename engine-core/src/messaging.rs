use std::{sync::mpsc, thread};

use crate::{
    board::Board,
    out,
    searching::{self, StopToken},
    uci::{self, GoMode, TimeControl},
};

pub enum EngineEvent {
    Uci(UciCommand),
    Search(SearchEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UciCommand {
    NewGame,
    Position(String),
    Go(String),
    Stop,
    Quit,
    Ping(u64),
}

#[derive(Debug, PartialEq, Eq)]
pub enum SearchEvent {
    BestMove { id: u64, mv: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineResponse {
    Pong(u64),
}

pub struct EngineWorkerHandler {
    pub engine_events_tx: mpsc::Sender<EngineEvent>,
    pub engine_respones_rx: mpsc::Receiver<EngineResponse>,
    pub join: std::thread::JoinHandle<()>,
}

const DEFAULT_DEPTH: u32 = 6;

pub fn spawn_worker() -> EngineWorkerHandler {
    let (ev_tx, ev_rx) = mpsc::channel::<EngineEvent>();
    let (engine_res_tx, engine_res_rx) = mpsc::channel::<EngineResponse>();

    let ev_tx_clone = ev_tx.clone();

    let join = std::thread::spawn(move || {
        let mut board: Board = Board::get_start_position();

        let stop_token = StopToken::new();
        let mut search_thread: Option<thread::JoinHandle<()>> = None;

        let stop_search = |stop: &StopToken, search_thread: &mut Option<thread::JoinHandle<()>>| {
            if search_thread.is_some() {
                stop.request_stop();

                if let Some(h) = search_thread.take() {
                    let _ = h.join();
                }
            }
        };

        let mut current_search_id = 0;

        loop {
            let cmd = match ev_rx.recv() {
                Ok(cmd) => cmd,
                Err(_) => break,
            };

            match cmd {
                EngineEvent::Uci(UciCommand::Ping(id)) => {
                    engine_res_tx.send(EngineResponse::Pong(id)).ok();
                }
                EngineEvent::Uci(UciCommand::NewGame) => {
                    stop_search(&stop_token, &mut search_thread);
                    board = Board::get_start_position();
                }
                EngineEvent::Uci(UciCommand::Position(pos_cmd)) => {
                    stop_search(&stop_token, &mut search_thread);
                    match uci::parse_uci_position_command(&pos_cmd) {
                        Ok(b) => board = b,
                        Err(_) => {
                            out::write_line("bestmove 0000");
                        }
                    }
                }
                EngineEvent::Uci(UciCommand::Go(go_cmd)) => {
                    stop_search(&stop_token, &mut search_thread);

                    stop_token.reset();

                    current_search_id += 1;
                    let search_id = current_search_id;

                    let ev_tx = ev_tx.clone();

                    let mut b = board.clone();
                    let stop = stop_token.clone();

                    let handle = thread::spawn(move || {
                        let moving_side = b.game_state.side_to_move;
                        let go_cmd =
                            uci::parse_uci_go_commmand(&go_cmd)
                                .ok()
                                .unwrap_or(uci::UciGoCommand {
                                    mode: uci::GoMode::Depth(5),
                                    tc: TimeControl::default(),
                                    search_moves: None,
                                    nodes: None,
                                    mate: None,
                                });
                        let depth = if let GoMode::Depth(depth) = go_cmd.mode {
                            depth
                        } else {
                            DEFAULT_DEPTH
                        };

                        let mv = searching::search_bestmove(&mut b, depth, &stop);
                        let mv_str = match mv {
                            Some(mv) => uci::serialize_move_to_uci_str(mv, moving_side),
                            None => "0000".to_string(),
                        };

                        ev_tx
                            .send(EngineEvent::Search(SearchEvent::BestMove {
                                id: search_id,
                                mv: mv_str,
                            }))
                            .ok();
                    });

                    search_thread = Some(handle);
                }
                EngineEvent::Uci(UciCommand::Stop) => {
                    if search_thread.is_none() {
                        out::write_line("bestmove 0000");
                        continue;
                    }

                    stop_token.request_stop();

                    if let Some(h) = search_thread.take() {
                        let _ = h.join();
                    }
                }
                EngineEvent::Uci(UciCommand::Quit) => {
                    stop_search(&stop_token, &mut search_thread);
                    break;
                }
                EngineEvent::Search(SearchEvent::BestMove { id, mv }) => {
                    if id != current_search_id {
                        continue;
                    }

                    out::write_line(&format!("bestmove {mv}"));
                }
            }
        }
    });

    EngineWorkerHandler {
        engine_events_tx: ev_tx_clone,
        engine_respones_rx: engine_res_rx,
        join: join,
    }
}
