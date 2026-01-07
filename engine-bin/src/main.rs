use std::{
    io::{BufRead, Write},
    sync::mpsc,
    time::Duration,
};

use engine_core::messaging::{WorkerCmd, WorkerEvent};

const ENGINE_NAME: &str = "Orion";
const AUTHOR_NAME: &str = "Voyager";

fn poll_engine_worker_events(
    event_rx: &mpsc::Receiver<WorkerEvent>,
    stdout: &mut impl Write,
) -> std::io::Result<()> {
    loop {
        match event_rx.try_recv() {
            Ok(WorkerEvent::BestMove(mv)) => {
                writeln!(stdout, "bestmove {}", mv)?;
                stdout.flush()?;
            }
            Ok(WorkerEvent::Info(s)) => {
                writeln!(stdout, "{s}")?;
                stdout.flush()?;
            }
            Ok(WorkerEvent::Pong(_)) => {}
            Err(mpsc::TryRecvError::Empty) => break,
            Err(mpsc::TryRecvError::Disconnected) => break,
        }
    }
    Ok(())
}

fn main() {
    let mut stdout = std::io::stdout();
    let stdin_rx = spawn_stdin_reader();

    let engine_worker_handler = engine_core::messaging::spawn_worker();

    let mut ping_id: u64 = 1;

    loop {
        poll_engine_worker_events(&engine_worker_handler.event_rx, &mut stdout).ok();

        let line = match stdin_rx.recv_timeout(Duration::from_millis(10)) {
            Ok(s) => s,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        let line = line.trim().to_string();

        if line.is_empty() {
            continue;
        }

        if line == "uci" {
            writeln!(stdout, "id name {}", ENGINE_NAME).ok();
            writeln!(stdout, "id author {}", AUTHOR_NAME).ok();
            writeln!(stdout, "uciok").ok();
            stdout.flush().ok();
            continue;
        }

        if line == "isready" {
            ping_id = ping_id.wrapping_add(1);
            let id = ping_id;

            engine_worker_handler.cmd_tx.send(WorkerCmd::Ping(id)).ok();

            loop {
                match engine_worker_handler
                    .event_rx
                    .recv_timeout(Duration::from_millis(200))
                {
                    Ok(WorkerEvent::Pong(x)) if x == id => {
                        writeln!(stdout, "readyok").ok();
                        stdout.flush().ok();
                        break;
                    }
                    Ok(WorkerEvent::BestMove(mv)) => {
                        writeln!(stdout, "bestmove {}", mv).ok();
                        stdout.flush().ok();
                    }
                    Ok(WorkerEvent::Info(s)) => {
                        writeln!(stdout, "{s}").ok();
                        stdout.flush().ok();
                    }
                    Ok(WorkerEvent::Pong(_)) => {}
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        continue;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }

            continue;
        }

        if line == "ucinewgame" {
            engine_worker_handler
                .cmd_tx
                .send(WorkerCmd::UciNewGame)
                .ok();
            continue;
        }

        if line.starts_with("position ") {
            engine_worker_handler
                .cmd_tx
                .send(WorkerCmd::Position(line))
                .ok();
            continue;
        }

        if line.starts_with("go") {
            engine_worker_handler.cmd_tx.send(WorkerCmd::Go(line)).ok();
            continue;
        }

        if line == "stop" {
            let _ = engine_worker_handler.cmd_tx.send(WorkerCmd::Stop);

            loop {
                match engine_worker_handler.event_rx.recv() {
                    Ok(WorkerEvent::BestMove(mv)) => {
                        writeln!(stdout, "bestmove {}", mv).ok();
                        stdout.flush().ok();
                        break;
                    }
                    Ok(WorkerEvent::Info(s)) => {
                        writeln!(stdout, "{s}").ok();
                        stdout.flush().ok();
                    }
                    Ok(WorkerEvent::Pong(_)) => {}
                    Err(_) => return,
                }
            }
            continue;
        }

        if line == "quit" {
            engine_worker_handler.cmd_tx.send(WorkerCmd::Quit).ok();
            break;
        }
    }

    let _ = engine_worker_handler.join.join().ok();
}

fn spawn_stdin_reader() -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel::<String>();

    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(s) => s.trim().to_string(),
                Err(_) => break,
            };
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    rx
}
