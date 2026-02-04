use std::{
    mem::MaybeUninit,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    time::Instant,
};

use crate::{
    board::Board,
    chess_consts,
    enums::{Move, Square},
    evaluation::{self},
    messaging::{EngineEvent, SearchEval, SearchEvent},
    move_generator::MoveBuffer,
    move_ordering,
    tt::{ProbeResult, TTEntryBound, TranspositionTable},
    uci, zobrist_hashing,
};

const INFINITY: i32 = 1_000_000_00;

const ONLY_CAPTURES_DEPTH: u32 = 2;

const FULL_DEPTH_MOVES: usize = 5;
const REDUCTION_DEPTH: usize = 4;

const NULL_MOVE_PRUNING_DEPTH: usize = 4;
const NULL_MOVE_REDUCTION_FACTOR: usize = 2;

const ASPIRATION_WINDOW_SIZE: usize = 30;

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

#[derive(Clone, Copy)]
pub(crate) struct NegamaxLimits {
    nodes_count: Option<u64>,
}

pub(crate) enum NegamaxResult {
    Completed(i32),
    Stopped,
}

pub(crate) fn negamax_ab(
    board: &mut Board,
    depth: u32,
    alpha: i32,
    beta: i32,
    ply: u32,
    search_state: &mut SearchState,
    tt: &mut TranspositionTable,
    bufs: &mut [MoveBuffer],
    limits: NegamaxLimits,
    stop_token: &StopToken,
) -> NegamaxResult {
    if stop_token.is_stopped()
        || matches!(limits.nodes_count, Some(nodes_limit) if NODES_COUNTER.load(Ordering::Relaxed) >= nodes_limit as usize)
    {
        return NegamaxResult::Stopped;
    }

    if board.game_state.half_move_clock >= 100 {
        NODES_COUNTER.fetch_add(1, Ordering::Relaxed);

        return NegamaxResult::Completed(0);
    }

    if board.is_repetition() {
        NODES_COUNTER.fetch_add(1, Ordering::Relaxed);

        return NegamaxResult::Completed(0);
    }

    let probe_result = tt.probe(
        board.game_state.zobrist_key,
        depth as u8,
        ply as u8,
        alpha,
        beta,
    );

    let current_rep_count = board.history.get_repetition_count(
        board.game_state.zobrist_key,
        board.game_state.half_move_clock,
    );
    let is_draw_candidate = current_rep_count >= 1 || board.game_state.half_move_clock >= 98;

    if let ProbeResult::Hit { score, mv, .. } = probe_result
        && !is_draw_candidate
    {
        if let Some(tt_move) = mv {
            board.make_move(tt_move);

            let is_draw = board.is_repetition() || board.game_state.half_move_clock >= 100;

            board.unmake_move();

            if !is_draw {
                return NegamaxResult::Completed(score);
            }
        } else {
            return NegamaxResult::Completed(score);
        }
    }

    let tt_move = match probe_result {
        ProbeResult::Hit { mv, .. } | ProbeResult::PartialHit { mv } => mv,
        ProbeResult::Miss => None,
    };

    let side_to_move = board.game_state.side_to_move;
    let is_in_check = board.is_in_check(side_to_move);

    if depth >= NULL_MOVE_PRUNING_DEPTH as u32 && !is_in_check {
        let prev_state = board.game_state;

        if zobrist_hashing::need_to_hash_enpassant(board) {
            board.game_state.zobrist_key ^= zobrist_hashing::get_enpassant_key(
                board.game_state.en_passant_square.unwrap().file(),
            );
        }

        board.change_side_to_move();
        board.game_state.en_passant_square = None;

        let null_score = match negamax_ab(
            board,
            depth - 1 - NULL_MOVE_REDUCTION_FACTOR as u32,
            -beta,
            -beta + 1,
            ply + 1,
            search_state,
            tt,
            bufs,
            limits,
            stop_token,
        ) {
            NegamaxResult::Completed(score) => -score,
            NegamaxResult::Stopped => {
                board.game_state = prev_state;

                return NegamaxResult::Stopped;
            }
        };

        board.game_state = prev_state;

        if null_score >= beta {
            tt.store(
                board.game_state.zobrist_key,
                depth as u8,
                ply as u8,
                null_score as i16,
                TTEntryBound::FailHigh,
                None,
            );

            return NegamaxResult::Completed(null_score);
        }
    }

    let (cur, rest) = bufs.split_first_mut().unwrap();
    cur.clear();
    board.generate_all_legal_moves(side_to_move, cur);

    if cur.len() == 0 {
        NODES_COUNTER.fetch_add(1, Ordering::Relaxed);

        if board.is_in_check(side_to_move) {
            let mate_score = -evaluation::MATE_EVALUATION + ply as i32;

            tt.store(
                board.game_state.zobrist_key,
                depth as u8,
                ply as u8,
                mate_score as i16,
                TTEntryBound::Exact,
                None,
            );

            return NegamaxResult::Completed(mate_score);
        } else {
            tt.store(
                board.game_state.zobrist_key,
                depth as u8,
                ply as u8,
                0,
                TTEntryBound::Exact,
                None,
            );

            return NegamaxResult::Completed(0);
        }
    }

    if depth == 0 {
        let q_eval_score =
            evaluation::quiescence_search(board, alpha, beta, bufs, ply, &search_state);

        let tt_bound = if q_eval_score >= beta {
            TTEntryBound::FailHigh
        } else if q_eval_score <= alpha {
            TTEntryBound::FailLow
        } else {
            TTEntryBound::Exact
        };

        tt.store(
            board.game_state.zobrist_key,
            0,
            ply as u8,
            q_eval_score as i16,
            tt_bound,
            None,
        );

        return NegamaxResult::Completed(q_eval_score);
    }

    NODES_COUNTER.fetch_add(1, Ordering::Relaxed);
    search_state.pv_clear(ply);

    let sort_quiet_moves = if depth > ONLY_CAPTURES_DEPTH as u32 {
        true
    } else {
        false
    };
    move_ordering::sort_moves(cur, ply, sort_quiet_moves, tt_move, search_state);

    let mut best_score = -INFINITY;
    let mut best_move = None;

    for (i, mv) in cur.iter().copied().enumerate() {
        let cur_alpha = best_score.max(alpha);

        if stop_token.is_stopped() {
            return NegamaxResult::Stopped;
        }

        let score = match search_move(
            SearchMoveContext {
                board: board,
                search_state: search_state,
                tt: tt,
                bufs: rest,
                stop_token: stop_token,
            },
            SearchMoveArgs {
                mv: mv,
                move_index: i,
                depth: depth,
                alpha: cur_alpha,
                beta: beta,
                ply: ply,
                limits: limits,
                is_in_check: is_in_check,
            },
        ) {
            SearchMoveResult::Ok(score) => score,
            SearchMoveResult::Stopped => return NegamaxResult::Stopped,
        };

        if score > best_score {
            best_score = score;
            best_move = Some(mv);

            search_state.pv_update(ply, mv);
        }

        if score >= beta {
            if !mv.is_capture() && !mv.is_promo() {
                search_state.update_killers(mv, ply);
                search_state.update_history(mv, depth);
            }

            tt.store(
                board.game_state.zobrist_key,
                depth as u8,
                ply as u8,
                score as i16,
                TTEntryBound::FailHigh,
                Some(mv),
            );
            return NegamaxResult::Completed(score);
        }
    }

    let bound = if best_score <= alpha {
        TTEntryBound::FailLow
    } else {
        TTEntryBound::Exact
    };

    tt.store(
        board.game_state.zobrist_key,
        depth as u8,
        ply as u8,
        best_score as i16,
        bound,
        best_move,
    );

    return NegamaxResult::Completed(best_score);
}

#[allow(dead_code)]
pub(crate) enum SearchBestMoveResult {
    Ok { mv: Move, score: i32 },
    FailLow { score: i32 },
    FailHigh { score: i32 },
    NoMoves,
    Stopped,
}

pub(crate) struct SearchBestMoveLimits {
    pub(crate) search_moves: Option<Vec<Move>>,
    pub(crate) nodes_limit: Option<u64>,
}

pub(crate) fn search_best_move(
    board: &mut Board,
    alpha: i32,
    beta: i32,
    depth: u32,
    search_state: &mut SearchState,
    tt: &mut TranspositionTable,
    bufs: &mut [Vec<Move>],
    limits: &SearchBestMoveLimits,
    stop: &StopToken,
) -> SearchBestMoveResult {
    if stop.is_stopped() {
        return SearchBestMoveResult::Stopped;
    }

    NODES_COUNTER.store(0, Ordering::Relaxed);
    search_state.pv_len.fill(0);

    let probe_result = tt.probe(board.game_state.zobrist_key, depth as u8, 0, alpha, beta);

    let current_rep_count = board.history.get_repetition_count(
        board.game_state.zobrist_key,
        board.game_state.half_move_clock,
    );
    let is_draw_candidate = current_rep_count >= 1 || board.game_state.half_move_clock >= 98;

    if let ProbeResult::Hit { score, mv, bound } = probe_result
        && !is_draw_candidate
    {
        if let Some(tt_move) = mv {
            board.make_move(tt_move);

            let is_draw = board.is_repetition() || board.game_state.half_move_clock >= 100;

            board.unmake_move();

            if !is_draw {
                match bound {
                    TTEntryBound::Exact => {
                        reconstruct_pv_from_tt(board, depth, search_state, tt, tt_move);
                        return SearchBestMoveResult::Ok {
                            mv: tt_move,
                            score: score,
                        };
                    }
                    TTEntryBound::FailLow => {
                        return SearchBestMoveResult::FailLow { score };
                    }
                    TTEntryBound::FailHigh => {
                        return SearchBestMoveResult::FailHigh { score };
                    }
                }
            }
        }
    }

    let tt_move = match probe_result {
        ProbeResult::Hit { mv, .. } | ProbeResult::PartialHit { mv } => mv,
        ProbeResult::Miss => None,
    };

    let side = board.game_state.side_to_move;
    let is_in_check = board.is_in_check(side);

    let (cur, rest) = bufs.split_first_mut().unwrap();
    cur.clear();
    board.generate_all_legal_moves(side, cur);
    if let Some(search_moves) = &limits.search_moves {
        cur.retain(|mv| search_moves.contains(mv));
    }

    if cur.len() == 0 {
        return SearchBestMoveResult::NoMoves;
    }

    let sort_quiet_moves = if depth > ONLY_CAPTURES_DEPTH {
        true
    } else {
        false
    };
    move_ordering::sort_moves(cur, 0, sort_quiet_moves, tt_move, search_state);

    let negamax_limits = NegamaxLimits {
        nodes_count: limits.nodes_limit,
    };

    let mut best_score = -INFINITY;

    for (i, mv) in cur.iter().copied().enumerate() {
        let cur_alpha = alpha.max(best_score);

        if stop.is_stopped() {
            return SearchBestMoveResult::Stopped;
        }

        let score = match search_move(
            SearchMoveContext {
                board: board,
                search_state: search_state,
                tt: tt,
                bufs: rest,
                stop_token: stop,
            },
            SearchMoveArgs {
                mv: mv,
                move_index: i,
                depth: depth,
                alpha: cur_alpha,
                beta: beta,
                ply: 0,
                limits: negamax_limits,
                is_in_check: is_in_check,
            },
        ) {
            SearchMoveResult::Ok(score) => score,
            SearchMoveResult::Stopped => return SearchBestMoveResult::Stopped,
        };

        if score > best_score {
            best_score = score;

            search_state.pv_update(0, mv);
        }

        if score >= beta {
            tt.store(
                board.game_state.zobrist_key,
                depth as u8,
                0,
                score as i16,
                TTEntryBound::FailHigh,
                Some(mv),
            );

            return SearchBestMoveResult::FailHigh { score: score };
        }
    }

    if best_score <= alpha {
        let best_mv = if search_state.pv_len[0] == 0 {
            None
        } else {
            unsafe { Some(search_state.pv_table[0][0].assume_init_read()) }
        };

        tt.store(
            board.game_state.zobrist_key,
            depth as u8,
            0,
            best_score as i16,
            TTEntryBound::FailLow,
            best_mv,
        );
        return SearchBestMoveResult::FailLow { score: best_score };
    }

    if search_state.pv_len[0] == 0 {
        return SearchBestMoveResult::NoMoves;
    }

    let mv = unsafe { search_state.pv_table[0][0].assume_init_read() };
    search_state.prev_pv_first_move = Some(mv);

    tt.store(
        board.game_state.zobrist_key,
        depth as u8,
        0,
        best_score as i16,
        TTEntryBound::Exact,
        Some(mv),
    );

    SearchBestMoveResult::Ok {
        mv: mv,
        score: best_score,
    }
}

fn reconstruct_pv_from_tt(
    board: &mut Board,
    depth: u32,
    search_state: &mut SearchState,
    tt: &TranspositionTable,
    first_move: Move,
) {
    search_state.pv_table[0][0].write(first_move);
    search_state.pv_len[0] = 1;

    board.make_move(first_move);

    let mut ply = 1;

    while ply < depth as usize && ply < chess_consts::MAX_PLY {
        let remaining_depth = (depth - ply as u32) as u8;
        if remaining_depth == 0 {
            break;
        }

        match tt.probe(
            board.game_state.zobrist_key,
            remaining_depth,
            ply as u8,
            -INFINITY,
            INFINITY,
        ) {
            ProbeResult::Hit {
                mv: Some(mv),
                bound: TTEntryBound::Exact,
                ..
            } => {
                search_state.pv_table[0][ply].write(mv);
                search_state.pv_len[0] += 1;
                board.make_move(mv);
                ply += 1;
            }
            _ => break,
        }
    }

    for _ in 0..ply {
        board.unmake_move();
    }
}

struct SearchMoveContext<'a> {
    board: &'a mut Board,
    search_state: &'a mut SearchState,
    tt: &'a mut TranspositionTable,
    bufs: &'a mut [Vec<Move>],
    stop_token: &'a StopToken,
}

struct SearchMoveArgs {
    mv: Move,
    move_index: usize,
    depth: u32,
    alpha: i32,
    beta: i32,
    ply: u32,
    limits: NegamaxLimits,
    is_in_check: bool,
}

enum SearchMoveResult {
    Ok(i32),
    Stopped,
}

fn search_move(
    SearchMoveContext {
        board,
        search_state,
        tt,
        bufs,
        stop_token,
    }: SearchMoveContext,
    SearchMoveArgs {
        mv,
        move_index,
        depth,
        alpha,
        beta,
        ply,
        limits,
        is_in_check,
    }: SearchMoveArgs,
) -> SearchMoveResult {
    board.make_move(mv);

    let is_pv_move = move_index == 0;

    let score = if is_pv_move {
        let s = match negamax_ab(
            board,
            depth - 1,
            -beta,
            -alpha,
            ply + 1,
            search_state,
            tt,
            bufs,
            limits,
            stop_token,
        ) {
            NegamaxResult::Completed(s) => -s,
            NegamaxResult::Stopped => {
                board.unmake_move();
                return SearchMoveResult::Stopped;
            }
        };

        s
    } else {
        let full_depth = depth - 1;
        let mut reduced_depth = full_depth;
        let gives_check = board.is_in_check(board.game_state.side_to_move);

        if can_reduce_depth(
            mv,
            move_index,
            depth,
            is_in_check,
            gives_check,
            is_pv_move,
            search_state,
            ply,
        ) {
            reduced_depth -= get_depth_reduction(depth);
        }

        let prob_score = match negamax_ab(
            board,
            reduced_depth,
            -alpha - 1,
            -alpha,
            ply + 1,
            search_state,
            tt,
            bufs,
            limits,
            stop_token,
        ) {
            NegamaxResult::Completed(score) => -score,
            NegamaxResult::Stopped => {
                board.unmake_move();
                return SearchMoveResult::Stopped;
            }
        };

        if prob_score > alpha {
            let s = match negamax_ab(
                board,
                full_depth,
                -beta,
                -alpha,
                ply + 1,
                search_state,
                tt,
                bufs,
                limits,
                stop_token,
            ) {
                NegamaxResult::Completed(s) => -s,
                NegamaxResult::Stopped => {
                    board.unmake_move();
                    return SearchMoveResult::Stopped;
                }
            };

            s
        } else {
            prob_score
        }
    };

    board.unmake_move();

    SearchMoveResult::Ok(score)
}

fn can_reduce_depth(
    mv: Move,
    move_index: usize,
    depth: u32,
    is_in_check: bool,
    gives_check: bool,
    is_pv_move: bool,
    search_state: &SearchState,
    ply: u32,
) -> bool {
    return move_index >= FULL_DEPTH_MOVES
        && depth >= REDUCTION_DEPTH as u32
        && mv.is_quiet()
        && !is_in_check
        && !is_pv_move
        && !gives_check
        && !search_state.is_mv_killer(mv, ply);
}

fn get_depth_reduction(depth: u32) -> u32 {
    if depth < REDUCTION_DEPTH as u32 {
        0
    } else if depth < 7 {
        1
    } else {
        2
    }
}

pub(crate) fn iterative_deepening_search(
    board: &mut Board,
    depth: u32,
    search_state: &mut SearchState,
    tt: &mut TranspositionTable,
    limits: SearchBestMoveLimits,
    out_tx: &mpsc::Sender<EngineEvent>,
    stop: &StopToken,
) -> Option<Move> {
    search_state.reset_for_new_search();
    NODES_COUNTER.store(0, Ordering::Relaxed);

    let mut best_move = None;
    let mut start_time;

    let mut bufs: Vec<MoveBuffer> = (0..chess_consts::MAX_PLY)
        .map(|_| Vec::with_capacity(chess_consts::MOVES_BUF_SIZE))
        .collect();

    let mut alpha = -INFINITY;
    let mut beta = INFINITY;

    let mut d = 1;
    let mut delta = ASPIRATION_WINDOW_SIZE;
    let mut attempts = 0;

    while d <= depth {
        start_time = Instant::now();

        let res = search_best_move(
            board,
            alpha,
            beta,
            d,
            search_state,
            tt,
            &mut bufs,
            &limits,
            stop,
        );

        let elapsed_ms = start_time.elapsed().as_millis();

        match res {
            SearchBestMoveResult::Ok { mv, score } => {
                best_move = Some(mv);

                let mut pv_str = String::new();

                let eval = if score > evaluation::MATE_SCORE {
                    SearchEval::Mate((evaluation::MATE_EVALUATION - score + 1) / 2)
                } else if score < -evaluation::MATE_SCORE {
                    SearchEval::Mate(-((evaluation::MATE_EVALUATION + score + 1) / 2))
                } else {
                    SearchEval::Score(score)
                };

                for i in 0..search_state.pv_len[0] {
                    pv_str.push_str(&format!(
                        "{}{}",
                        if i == 0 { "" } else { " " },
                        uci::serialize_move_to_uci_str(unsafe {
                            search_state.pv_table[0][i].assume_init_read()
                        })
                    ));
                }

                let nodes_count = NODES_COUNTER.load(Ordering::Relaxed);
                let nps = if elapsed_ms > 0 {
                    nodes_count * 1000 / elapsed_ms as usize
                } else {
                    0
                };

                out_tx
                    .send(EngineEvent::Search(SearchEvent::Info {
                        depth: d,
                        eval: eval,
                        nodes: NODES_COUNTER.load(Ordering::Relaxed),
                        time: elapsed_ms,
                        nps: nps,
                        pv_string: pv_str,
                    }))
                    .ok();

                alpha = score - ASPIRATION_WINDOW_SIZE as i32;
                beta = score + ASPIRATION_WINDOW_SIZE as i32;

                d += 1;
                delta = ASPIRATION_WINDOW_SIZE;
                attempts = 0;
            }
            SearchBestMoveResult::FailLow { .. } => {
                attempts += 1;

                if attempts > 3 {
                    alpha = -INFINITY;
                    continue;
                }

                delta *= 2;
                alpha -= delta as i32;
            }
            SearchBestMoveResult::FailHigh { .. } => {
                attempts += 1;

                if attempts > 3 {
                    beta = INFINITY;
                    continue;
                }

                delta *= 2;
                beta += delta as i32;
            }
            SearchBestMoveResult::NoMoves | SearchBestMoveResult::Stopped => {
                break;
            }
        }
    }

    best_move
}

pub(crate) struct SearchState {
    pub(crate) killers: [[Option<Move>; chess_consts::MAX_PLY]; 2],
    pub(crate) history: [[u64; chess_consts::SQUARES_COUNT]; chess_consts::SQUARES_COUNT],
    pub(crate) pv_len: [usize; chess_consts::MAX_PLY],
    pub(crate) pv_table: [[MaybeUninit<Move>; chess_consts::MAX_PLY]; chess_consts::MAX_PLY],
    pub(crate) prev_pv_first_move: Option<Move>,
}

impl SearchState {
    pub(crate) fn new() -> SearchState {
        SearchState {
            killers: [[None; chess_consts::MAX_PLY]; 2],
            history: [[0; chess_consts::SQUARES_COUNT]; chess_consts::SQUARES_COUNT],
            pv_len: [0; chess_consts::MAX_PLY],
            pv_table: [[MaybeUninit::uninit(); chess_consts::MAX_PLY]; chess_consts::MAX_PLY],
            prev_pv_first_move: None,
        }
    }

    pub(crate) fn reset_for_new_search(&mut self) {
        self.killers.fill([None; chess_consts::MAX_PLY]);

        for from in Square::all() {
            for to in Square::all() {
                self.history[from.index() as usize][to.index() as usize] >>= 1;
            }
        }

        self.pv_len.fill(0);

        self.prev_pv_first_move = None;
    }

    fn update_killers(&mut self, mv: Move, ply: u32) {
        let pl = ply as usize;

        if self.killers[0][pl] == Some(mv) {
            return;
        }

        self.killers[1][pl] = self.killers[0][pl];
        self.killers[0][pl] = Some(mv);
    }

    fn is_mv_killer(&self, mv: Move, ply: u32) -> bool {
        let p = ply as usize;

        if self.killers[0][p] == Some(mv) {
            return true;
        }

        if self.killers[1][p] == Some(mv) {
            return true;
        }

        false
    }

    fn update_history(&mut self, mv: Move, depth: u32) {
        let (from, to) = mv.get_from_to();
        let f = from.index() as usize;
        let t = to.index() as usize;
        let add = (depth * depth) as u64;

        self.history[f][t] = self.history[f][t].saturating_add(add);
    }

    fn pv_clear(&mut self, ply: u32) {
        self.pv_len[ply as usize] = 0;
    }

    fn pv_update(&mut self, ply: u32, mv: Move) {
        let pl = ply as usize;

        self.pv_table[pl][0].write(mv);

        for i in 0..self.pv_len[pl + 1] {
            self.pv_table[pl][i + 1].write(unsafe { self.pv_table[pl + 1][i].assume_init_read() });
        }

        self.pv_len[pl] = self.pv_len[pl + 1] + 1;
    }
}

#[cfg(test)]
mod tests {
    use crate::{fen_parser, init};

    use super::*;

    #[test]
    #[ignore]
    fn test_nodes_count() {
        init::init_engine();

        let mut board =
            fen_parser::parse_fen_string(chess_consts::fen_strings::KILLER_POS_FEN).unwrap();
        let mut search_state = SearchState::new();
        let mut tt = TranspositionTable::new(1);

        let mut bufs: Vec<MoveBuffer> = (0..chess_consts::MAX_PLY)
            .map(|_| Vec::with_capacity(chess_consts::MOVES_BUF_SIZE))
            .collect();

        let _ = search_best_move(
            &mut board,
            -INFINITY,
            INFINITY,
            6,
            &mut search_state,
            &mut tt,
            &mut bufs,
            &SearchBestMoveLimits {
                nodes_limit: None,
                search_moves: None,
            },
            &StopToken::new(),
        );

        println!("Nodes count: {}", NODES_COUNTER.load(Ordering::Relaxed));
    }
}
