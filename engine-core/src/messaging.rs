use std::{
    sync::mpsc,
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    board::Board,
    enums::Side,
    init::init_engine,
    out,
    searching::{self, SearchBestMoveLimits, SearchState, StopToken},
    tt::TranspositionTable,
    uci::{self, TimeControl, UciGoCommand},
};

const DEFAULT_TT_SIZE: usize = 64;

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
    BestMove {
        id: u64,
        mv: String,
    },
    Info {
        depth: u32,
        eval: SearchEval,
        nodes: usize,
        time: u128,
        nps: usize,
        pv_string: String,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum SearchEval {
    Score(i32),
    Mate(i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineResponse {
    Pong(u64),
}

pub struct EngineWorkerHandler {
    pub tx: mpsc::Sender<EngineEvent>,
    pub out_rx: mpsc::Receiver<EngineResponse>,
    pub join: std::thread::JoinHandle<()>,
}

pub fn spawn_worker() -> EngineWorkerHandler {
    let (tx, rx) = mpsc::channel::<EngineEvent>();
    let (out_tx, out_rx) = mpsc::channel::<EngineResponse>();

    let tx_clone = tx.clone();

    let join = std::thread::spawn(move || worker_loop(tx, rx, out_tx));

    EngineWorkerHandler {
        tx: tx_clone,
        out_rx,
        join: join,
    }
}

fn worker_loop(
    ev_tx: mpsc::Sender<EngineEvent>,
    ev_rx: mpsc::Receiver<EngineEvent>,
    out_tx: mpsc::Sender<EngineResponse>,
) {
    let mut board: Option<Board> = None;
    let search_worker = SearchWorker::start(ev_tx, StopToken::new());

    let mut current_search_id = 0;

    loop {
        let cmd = match ev_rx.recv() {
            Ok(cmd) => cmd,
            Err(_) => break,
        };

        match cmd {
            EngineEvent::Uci(UciCommand::Ping(id)) => {
                init_engine();

                out_tx.send(EngineResponse::Pong(id)).ok();
            }
            EngineEvent::Uci(UciCommand::NewGame) => {
                current_search_id += 1;
                search_worker.stop();

                board = Some(Board::get_start_position());
            }
            EngineEvent::Uci(UciCommand::Position(pos_cmd)) => {
                search_worker.stop();

                match uci::parse_uci_position_command(&pos_cmd) {
                    Ok(b) => board = Some(b),
                    Err(_) => {
                        out::write_line("bestmove 0000");
                    }
                }
            }
            EngineEvent::Uci(UciCommand::Go(go_cmd)) => {
                current_search_id += 1;
                search_worker.stop();

                let mut b = board.clone().unwrap_or_else(Board::get_start_position);

                let go_cmd = match uci::parse_uci_go_command(&go_cmd, &mut b) {
                    Ok(cmd) => cmd,
                    Err(msg) => {
                        out::write_line(&format!("info string error parsing go command: {msg}"));
                        continue;
                    }
                };

                search_worker.go(go_cmd, b, current_search_id);
            }
            EngineEvent::Uci(UciCommand::Stop) => {
                search_worker.stop();
            }
            EngineEvent::Uci(UciCommand::Quit) => {
                search_worker.stop();
                search_worker.quit();

                break;
            }
            EngineEvent::Search(SearchEvent::BestMove { id, mv }) => {
                if id != current_search_id {
                    continue;
                }

                out::write_line(&format!("bestmove {mv}"));
            }
            EngineEvent::Search(SearchEvent::Info {
                depth,
                eval,
                nodes,
                time,
                nps,
                pv_string,
            }) => {
                let mut info = format!("info depth {depth}");

                match eval {
                    SearchEval::Score(score) => info.push_str(&format!(" score cp {score}")),
                    SearchEval::Mate(distance) => info.push_str(&format!(" score mate {distance}")),
                }

                info.push_str(&format!(" nodes {nodes}"));

                info.push_str(&format!(" nps {nps}"));

                info.push_str(&format!(" time {time}"));

                info.push_str(&format!(" pv {pv_string}"));

                out::write_line(&info);
            }
        }
    }
}

enum SearchCmd {
    Go {
        go_cmd: UciGoCommand,
        board: Board,
        search_id: u64,
    },
    Quit,
}

struct SearchWorker {
    tx: mpsc::Sender<SearchCmd>,
    handle: JoinHandle<()>,
    stop: StopToken,
}

impl SearchWorker {
    fn start(result_tx: mpsc::Sender<EngineEvent>, stop: StopToken) -> SearchWorker {
        let (tx, rx) = mpsc::channel();
        let stop_cl = stop.clone();

        let handle = thread::spawn(move || SearchWorker::worker_loop(rx, result_tx, stop_cl));

        SearchWorker {
            tx: tx,
            handle: handle,
            stop: stop,
        }
    }

    fn go(&self, cmd: UciGoCommand, board: Board, search_id: u64) {
        self.stop.reset();

        let _ = self.tx.send(SearchCmd::Go {
            go_cmd: cmd,
            board: board,
            search_id: search_id,
        });
    }

    fn stop(&self) {
        self.stop.request_stop();
    }

    fn quit(self) {
        self.tx.send(SearchCmd::Quit).ok();
        self.handle.join().ok();
    }

    fn worker_loop(
        rx: mpsc::Receiver<SearchCmd>,
        out_tx: mpsc::Sender<EngineEvent>,
        stop: StopToken,
    ) {
        let mut search_state = SearchState::new();
        let mut tt = TranspositionTable::new(DEFAULT_TT_SIZE);

        loop {
            match rx.recv() {
                Ok(SearchCmd::Go {
                    go_cmd,
                    mut board,
                    search_id,
                }) => {
                    let depth = go_cmd.depth.unwrap_or(u32::MAX);

                    let max_time_ms = if go_cmd.infinite {
                        None
                    } else if let Some(move_time) = go_cmd.move_time {
                        Some(move_time)
                    } else {
                        SearchWorker::compute_move_time(&go_cmd.time, board.game_state.side_to_move)
                    };

                    if let Some(max_time_ms) = max_time_ms {
                        let safe_time_ms = max_time_ms.saturating_sub(50);
                        SearchWorker::start_timer(stop.clone(), safe_time_ms);
                    };

                    let limits = SearchBestMoveLimits {
                        nodes_limit: go_cmd.nodes,
                        search_moves: go_cmd.search_moves.clone(),
                    };

                    let search_res = searching::iterative_deepening_search(
                        &mut board,
                        depth,
                        &mut search_state,
                        &mut tt,
                        limits,
                        &out_tx,
                        &stop,
                    );
                    let Some(mv) = search_res else {
                        out_tx
                            .send(EngineEvent::Search(SearchEvent::BestMove {
                                id: search_id,
                                mv: "0000".to_string(),
                            }))
                            .ok();
                        continue;
                    };

                    let mv_str = uci::serialize_move_to_uci_str(mv);

                    out_tx
                        .send(EngineEvent::Search(SearchEvent::BestMove {
                            id: search_id,
                            mv: mv_str,
                        }))
                        .ok();
                }
                Ok(SearchCmd::Quit) | Err(_) => break,
            }
        }
    }

    fn start_timer(stop: StopToken, max_time_ms: u64) {
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(max_time_ms));

            stop.request_stop();
        });
    }

    fn compute_move_time(time_control: &TimeControl, side: Side) -> Option<u64> {
        let time_left = match side {
            Side::White => time_control.wtime,
            Side::Black => time_control.btime,
        }?;

        let moves_to_go = time_control.movestogo.unwrap_or(40) as u64; // default: 40 moves
        let inc = match side {
            Side::White => time_control.winc,
            Side::Black => time_control.binc,
        }
        .unwrap_or(0);

        Some(time_left / moves_to_go + inc)
    }
}
