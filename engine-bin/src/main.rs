use std::{io::BufRead, sync::mpsc, time::Duration};

use engine_core::{
    messaging::{EngineEvent, EngineResponse, UciCommand},
    out,
};

const ENGINE_NAME: &str = "Orion";
const AUTHOR_NAME: &str = "Voyager";

fn main() {
    out::init_out(std::io::stdout());

    let stdin = std::io::stdin();

    let engine_worker_handler = engine_core::messaging::spawn_worker();

    let mut ping_id: u64 = 1;

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(s) => s.trim().to_string(),
            Err(_) => break,
        };

        if line.is_empty() {
            continue;
        }

        if line == "uci" {
            out::write_line(&format!("id name {}", ENGINE_NAME));
            out::write_line(&format!("id author {}", AUTHOR_NAME));
            out::write_line("uciok");
            continue;
        }

        if line == "isready" {
            let id = ping_id;
            ping_id = ping_id.wrapping_add(1);

            engine_worker_handler
                .engine_events_tx
                .send(EngineEvent::Uci(UciCommand::Ping(id)))
                .ok();

            loop {
                match engine_worker_handler
                    .engine_respones_rx
                    .recv_timeout(Duration::from_millis(200))
                {
                    Ok(EngineResponse::Pong(x)) if x == id => {
                        out::write_line("readyok");
                        break;
                    }
                    Ok(EngineResponse::Pong(_)) => {
                        continue;
                    }
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
                .engine_events_tx
                .send(EngineEvent::Uci(UciCommand::NewGame))
                .ok();
            continue;
        }

        if line.starts_with("position ") {
            engine_worker_handler
                .engine_events_tx
                .send(EngineEvent::Uci(UciCommand::Position(line)))
                .ok();
            continue;
        }

        if line.starts_with("go") {
            engine_worker_handler
                .engine_events_tx
                .send(EngineEvent::Uci(UciCommand::Go(line)))
                .ok();
            continue;
        }

        if line == "stop" {
            let _ = engine_worker_handler
                .engine_events_tx
                .send(EngineEvent::Uci(UciCommand::Stop));

            continue;
        }

        if line == "quit" {
            engine_worker_handler
                .engine_events_tx
                .send(EngineEvent::Uci(UciCommand::Quit))
                .ok();
            break;
        }
    }

    let _ = engine_worker_handler.join.join().ok();
}
