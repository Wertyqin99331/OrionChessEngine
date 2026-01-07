use std::{
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

use crate::{board::Board, searching::StopToken, uci};
use rand::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerCmd {
    UciNewGame,
    Position(String),
    Go(String),
    Stop,
    Quit,
    Ping(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerEvent {
    BestMove(String),
    Info(String),
    Pong(u64),
}

pub struct EngineWorkerHandler {
    pub cmd_tx: mpsc::Sender<WorkerCmd>,
    pub event_rx: mpsc::Receiver<WorkerEvent>,
    pub join: std::thread::JoinHandle<()>,
}

pub fn spawn_worker() -> EngineWorkerHandler {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (ev_tx, ev_rx) = mpsc::channel();

    let join = std::thread::spawn(move || {
        let mut board: Board = Board::get_start_position();

        let stop_token = StopToken::new();
        let mut search_thread: Option<thread::JoinHandle<()>> = None;
        let mut search_result_rx: Option<mpsc::Receiver<String>> = None;

        let stop_search =
            |stop: &StopToken,
             search_thread: &mut Option<thread::JoinHandle<()>>,
             search_result_rx: &mut Option<mpsc::Receiver<String>>| {
                if search_thread.is_some() {
                    stop.request_stop();

                    if let Some(rx) = search_result_rx.take() {
                        let _ = rx.recv_timeout(Duration::from_millis(50));
                    }

                    if let Some(h) = search_thread.take() {
                        let _ = h.join();
                    }
                } else {
                    search_result_rx.take();
                }
            };

        let poll_search_result =
            |search_thread: &mut Option<thread::JoinHandle<()>>,
             search_result_rx: &mut Option<mpsc::Receiver<String>>| {
                let Some(rx) = search_result_rx.as_ref() else {
                    return;
                };

                match rx.try_recv() {
                    Ok(bm) => {
                        ev_tx.send(WorkerEvent::BestMove(bm)).ok();

                        if let Some(search_thread_handle) = search_thread.take() {
                            search_thread_handle.join().ok();
                        }

                        search_result_rx.take();
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                    _ => {
                        if let Some(search_thread_handle) = search_thread.take() {
                            search_thread_handle.join().ok();
                        }

                        search_result_rx.take();
                    }
                }
            };

        let cmd_timeout = Duration::from_millis(20);

        loop {
            poll_search_result(&mut search_thread, &mut search_result_rx);

            let cmd = match cmd_rx.recv_timeout(cmd_timeout) {
                Ok(cmd) => cmd,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                _ => break,
            };

            match cmd {
                WorkerCmd::Ping(id) => {
                    ev_tx.send(WorkerEvent::Pong(id)).ok();
                }
                WorkerCmd::UciNewGame => {
                    stop_search(&stop_token, &mut search_thread, &mut search_result_rx);
                    board = Board::get_start_position();
                }

                WorkerCmd::Position(pos_cmd) => {
                    stop_search(&stop_token, &mut search_thread, &mut search_result_rx);
                    match uci::parse_uci_position_command(&pos_cmd) {
                        Ok(b) => board = b,
                        Err(_) => {
                            ev_tx.send(WorkerEvent::BestMove("0000".to_string())).ok();
                        }
                    }
                }

                WorkerCmd::Go(go_cmd) => {
                    stop_token.reset();
                    stop_search(&stop_token, &mut search_thread, &mut search_result_rx);

                    let (res_tx, res_rx) = mpsc::channel::<String>();
                    let mut b = board.clone();
                    let stop_token = stop_token.clone();

                    let handle = thread::spawn(move || {
                        thread::sleep(Duration::from_millis(200));
                        let moving_side = b.game_state.side_to_move;
                        let _ = uci::parse_uci_go_commmand(&go_cmd).ok();

                        let moves = b.generate_legal_moves_to_vec(moving_side);

                        let mut rng = rand::rng();
                        let rnd_mv_index = rng.random_range(0..moves.len());
                        let mv = moves[rnd_mv_index];

                        let mv_str = uci::serialize_move_to_uci_str(mv, moving_side);
                        res_tx.send(mv_str).ok();
                    });

                    search_thread = Some(handle);
                    search_result_rx = Some(res_rx);
                }

                WorkerCmd::Stop => {
                    if search_thread.is_none() {
                        let _ = ev_tx.send(WorkerEvent::BestMove("0000".to_string())).ok();
                        continue;
                    }

                    stop_token.request_stop();

                    let bm = search_result_rx
                        .take()
                        .and_then(|rx| rx.recv().ok())
                        .unwrap_or_else(|| "0000".to_string());

                    if let Some(h) = search_thread.take() {
                        let _ = h.join();
                    }

                    let _ = ev_tx.send(WorkerEvent::BestMove(bm)).ok();
                }
                WorkerCmd::Quit => {
                    stop_search(&stop_token, &mut search_thread, &mut search_result_rx);
                    break;
                }
            }
        }
    });

    EngineWorkerHandler {
        cmd_tx: cmd_tx,
        event_rx: ev_rx,
        join: join,
    }
}
