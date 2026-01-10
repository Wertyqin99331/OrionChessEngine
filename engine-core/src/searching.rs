use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use rand::seq::SliceRandom;

use crate::{
    board::Board, chess_consts, enums::Move, evaluation, move_generator::MoveBuffer, move_sorting,
};

const INFINITY: i32 = 1_000_000_00;

pub(crate) static NODES_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
pub struct StopToken(Arc<AtomicBool>);

impl StopToken {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub fn request_stop(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn reset(&self) {
        self.0.store(false, Ordering::Relaxed);
    }

    pub fn is_stopped(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

pub(crate) fn negamax_ab(
    board: &mut Board,
    depth: u32,
    alpha: i32,
    beta: i32,
    ply: u32,
    stop_token: &StopToken,
    bufs: &mut [MoveBuffer],
) -> i32 {
    if board.game_state.half_move_clock >= 100 {
        NODES_COUNTER.fetch_add(1, Ordering::Relaxed);
        return 0;
    }

    if depth == 0 {
        return evaluation::quiescence_eval(board, alpha, beta, bufs);
    }

    NODES_COUNTER.fetch_add(1, Ordering::Relaxed);

    let (cur, rest) = bufs.split_first_mut().unwrap();

    cur.clear();

    let side_to_move = board.game_state.side_to_move;
    board.generate_all_legal_moves(side_to_move, cur);

    if cur.len() == 0 {
        if board.is_in_check(side_to_move) {
            return -evaluation::MATE_EVALUATION + ply as i32;
        } else {
            return 0;
        }
    }

    let mut best = -INFINITY;

    move_sorting::sort_moves(cur);

    for mv in cur.iter().copied() {
        let cur_alpha = best.max(alpha);

        if stop_token.is_stopped() {
            if best == -INFINITY {
                return alpha;
            }
            {
                return best;
            }
        }

        board.make_move(mv);

        let score = -negamax_ab(
            board,
            depth - 1,
            -beta,
            -cur_alpha,
            ply + 1,
            stop_token,
            rest,
        );

        board.unmake_move();

        if score > best {
            best = score;
        }

        if score >= beta {
            break;
        }
    }

    return best;
}

pub(crate) fn search_bestmove(board: &mut Board, depth: u32, stop: &StopToken) -> Option<Move> {
    NODES_COUNTER.store(0, Ordering::Relaxed);

    let side = board.game_state.side_to_move;

    let mut bufs: Vec<MoveBuffer> = (0..chess_consts::MAX_PLY)
        .map(|_| Vec::with_capacity(chess_consts::MOVES_BUF_SIZE))
        .collect();

    board.generate_all_legal_moves(side, &mut bufs[0]);

    let (cur, rest) = bufs.split_first_mut().unwrap();
    if cur.len() == 0 {
        return None;
    }

    let mut rng = rand::rng();
    cur.shuffle(&mut rng);

    let mut best_mv = cur[0];
    let mut best_score = -INFINITY;
    let mut alpha = -INFINITY;
    let beta = INFINITY;

    for mv in cur.iter().copied() {
        if stop.is_stopped() {
            break;
        }

        NODES_COUNTER.fetch_add(1, Ordering::Relaxed);

        board.make_move(mv);

        let score = -negamax_ab(board, depth - 1, -beta, -alpha, 1, stop, rest);

        board.unmake_move();

        if score > best_score {
            best_score = score;
            best_mv = mv;
        }

        if score > alpha {
            alpha = score;
        }
    }

    Some(best_mv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_nodes_count() {
        let mut board = Board::get_start_position();

        let _ = search_bestmove(&mut board, 7, &StopToken::new());

        println!("Nodes count: {}", NODES_COUNTER.load(Ordering::Relaxed));
    }
}
