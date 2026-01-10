use std::sync::atomic::Ordering;

use crate::{
    board::Board,
    chess_consts,
    enums::{Piece, Side},
    helpers,
    move_generator::MoveBuffer,
    searching,
};

pub(crate) const MATE_EVALUATION: i32 = 30_000;

mod piece_scores {

    use crate::enums::{Piece, Side};

    pub(super) const WHITE_PAWN_SCORE: i32 = 100;
    pub(super) const BLACK_PAWN_SCORE: i32 = -100;
    pub(super) const WHITE_KNIGHT_SCORE: i32 = 300;
    pub(super) const BLACK_KNIGHT_SCORE: i32 = -300;
    pub(super) const WHITE_BISHOP_SCORE: i32 = 350;
    pub(super) const BLACK_BISHOP_SCORE: i32 = -350;
    pub(super) const WHITE_ROOK_SCORE: i32 = 500;
    pub(super) const BLACK_ROOK_SCORE: i32 = -500;
    pub(super) const WHITE_QUEEN_SCORE: i32 = 1000;
    pub(super) const BLACK_QUEEN_SCORE: i32 = -1000;
    pub(super) const WHITE_KING_SCORE: i32 = 10_000;
    pub(super) const BLACK_KING_SCORE: i32 = -10_000;

    pub(super) fn get_piece_score(piece: Piece, side: Side) -> i32 {
        if side == Side::White {
            match piece {
                Piece::Pawn => WHITE_PAWN_SCORE,
                Piece::Knight => WHITE_KNIGHT_SCORE,
                Piece::Bishop => WHITE_BISHOP_SCORE,
                Piece::Rook => WHITE_ROOK_SCORE,
                Piece::Queen => WHITE_QUEEN_SCORE,
                Piece::King => WHITE_KING_SCORE,
            }
        } else {
            match piece {
                Piece::Pawn => BLACK_PAWN_SCORE,
                Piece::Knight => BLACK_KNIGHT_SCORE,
                Piece::Bishop => BLACK_BISHOP_SCORE,
                Piece::Rook => BLACK_ROOK_SCORE,
                Piece::Queen => BLACK_QUEEN_SCORE,
                Piece::King => BLACK_KING_SCORE,
            }
        }
    }
}

mod pst_tables {
    use crate::{
        chess_consts,
        enums::{Side, Square},
    };

    pub(super) fn get_pst_value(
        table: &[i16; chess_consts::SQUARES_COUNT],
        square: Square,
        side: Side,
    ) -> i16 {
        let index = if side == Side::White {
            square.index() as usize ^ 56
        } else {
            square.index() as usize
        };

        table[index]
    }

    #[rustfmt::skip]
    pub(super) const PAWN_PST_TABLE: [i16; chess_consts::SQUARES_COUNT] = [
     0,   0,   0,   0,   0,   0,   0,   0,
    30,  30,  30,  40,  40,  30,  30,  30,
    20,  20,  20,  30,  30,  30,  20,  20,
    10,  10,  10,  20,  20,  10,  10,  10,
     5,   5,  10,  20,  20,   5,   5,   5,
     0,   0,   0,   5,   5,   0,   0,   0,
     0,   0,   0, -10, -10,   0,   0,   0,
     0,   0,   0,   0,   0,   0,   0,   0
 ];

    #[rustfmt::skip]
    pub(super) const KNIGHT_PST_TABLE: [i16; chess_consts::SQUARES_COUNT] = [
     5,   0,   0,   0,   0,   0,   0,  -5,
    -5,   0,   0,  10,  10,   0,   0,  -5,
    -5,   5,  20,  20,  20,  20,   5,  -5,
    -5,  10,  20,  30,  30,  20,  10,  -5,
    -5,  10,  20,  30,  30,  20,  10,  -5,
    -5,   5,  20,  10,  10,  20,   5,  -5,
    -5,   0,   0,   0,   0,   0,   0,  -5,
    -5, -10,   0,   0,   0,   0, -10,  -5
     ];

    #[rustfmt::skip]
    pub(super) const BISHOP_PST_TABLE: [i16; chess_consts::SQUARES_COUNT] = [
     0,   0,   0,   0,   0,   0,   0,   0,
     0,   0,   0,   0,   0,   0,   0,   0,
     0,   0,   0,  10,  10,   0,   0,   0,
     0,   0,  10,  15,  15,  10,   0,   0,
     0,   0,  10,  15,  15,  10,   0,   0,
     0,  10,   0,   0,   0,   0,  10,   0,
     0,  15,   0,   0,   0,   0,  15,   0,
     0,   0, -10,   0,   0, -10,   0,   0
    ];

    #[rustfmt::skip]
    pub(super) const ROOK_PST_TABLE: [i16; chess_consts::SQUARES_COUNT] = [
    50,  50,  50,  50,  50,  50,  50,  50,
    50,  50,  50,  50,  50,  50,  50,  50,
     0,   0,  10,  20,  20,  10,   0,   0,
     0,   0,  10,  20,  20,  10,   0,   0,
     0,   0,  10,  20,  20,  10,   0,   0,
     0,   0,  10,  20,  20,  10,   0,   0,
     0,   0,  10,  20,  20,  10,   0,   0,
     0,   0,   0,  20,  20,   0,   0,   0
    ];

    #[rustfmt::skip]
    pub(super) const QUEEN_PST_TABLE: [i16; chess_consts::SQUARES_COUNT] = [
     -20,-10,-10, -5, -5,-10,-10,-20,
     -10,  0,  5,  0,  0,  0,  0,-10,
     -10,  5,  5,  5,  5,  5,  0,-10,
       0,  0,  5,  5,  5,  5,  0, -5,
      -5,  0,  5,  5,  5,  5,  0, -5,
     -10,  0,  5,  5,  5,  5,  0,-10,
     -10,  0,  0,  0,  0,  0,  0,-10,
     -20,-10,-10, -5, -5,-10,-10,-20
    ];

    #[rustfmt::skip]
    pub(super) const KING_MIDGAME_PST_TABLE: [i16; chess_consts::SQUARES_COUNT] = [
     -30,-40,-40,-50,-50,-40,-40,-30,
     -30,-40,-40,-50,-50,-40,-40,-30,
     -30,-40,-40,-50,-50,-40,-40,-30,
     -30,-40,-40,-50,-50,-40,-40,-30,
     -20,-30,-30,-40,-40,-30,-30,-20,
     -10,-20,-20,-20,-20,-20,-20,-10,
      20, 20,  0,  0,  0,  0, 20, 20,
      20, 30, 10,  0,  0, 10, 30, 20
    ];

    #[rustfmt::skip]
    pub(super) const KING_ENDGAME_PST_TABLE: [i16; chess_consts::SQUARES_COUNT] = [
     -50,-30,-30,-30,-30,-30,-30,-50,
     -30,-30,  0,  0,  0,  0,-30,-30,
     -30,-10, 20, 30, 30, 20,-10,-30,
     -30,-10, 30, 40, 40, 30,-10,-30,
     -30,-10, 30, 40, 40, 30,-10,-30,
     -30,-10, 20, 30, 30, 20,-10,-30,
     -30,-20,-10,  0,  0,-10,-20,-30,
     -50,-40,-30,-20,-20,-30,-40,-50
    ];
}

pub(crate) fn evalute(board: &Board, side: Side) -> i32 {
    let mut score: i32 = 0;
    let phase = calc_phase(board);

    for piece in Piece::all() {
        let white_bb = board.get_bb(Side::White, piece);
        let black_bb = board.get_bb(Side::Black, piece);

        score += white_bb.count_ones() as i32 * piece_scores::get_piece_score(piece, Side::White);
        score += black_bb.count_ones() as i32 * piece_scores::get_piece_score(piece, Side::Black);

        let pst_table = match piece {
            Piece::Pawn => pst_tables::PAWN_PST_TABLE,
            Piece::Knight => pst_tables::KNIGHT_PST_TABLE,
            Piece::Bishop => pst_tables::BISHOP_PST_TABLE,
            Piece::Rook => pst_tables::ROOK_PST_TABLE,
            Piece::Queen => pst_tables::QUEEN_PST_TABLE,
            Piece::King => {
                if (0..=10).contains(&phase) {
                    pst_tables::KING_ENDGAME_PST_TABLE
                } else {
                    pst_tables::KING_MIDGAME_PST_TABLE
                }
            }
        };

        for sq in helpers::get_squares_iter(white_bb) {
            score += pst_tables::get_pst_value(&pst_table, sq, Side::White) as i32;
        }

        for sq in helpers::get_squares_iter(black_bb) {
            score -= pst_tables::get_pst_value(&pst_table, sq, Side::Black) as i32;
        }
    }

    return if side == Side::White { score } else { -score };
}

pub(crate) fn quiescence_eval(
    board: &mut Board,
    mut alpha: i32,
    beta: i32,
    bufs: &mut [MoveBuffer],
) -> i32 {
    searching::NODES_COUNTER.fetch_add(1, Ordering::Relaxed);

    let eval_score = evalute_cur_side(&*board);

    if eval_score >= beta {
        return beta;
    }

    if eval_score > alpha {
        alpha = eval_score;
    }

    let moving_side = board.game_state.side_to_move;

    let (cur_buf, rest_bufs) = bufs.split_first_mut().unwrap();

    board.generate_legal_captures(moving_side, cur_buf);

    for mv in cur_buf.iter().copied() {
        board.make_move(mv);

        let score = -quiescence_eval(board, -beta, -alpha, rest_bufs);

        board.unmake_move();

        if score >= beta {
            return beta;
        }

        if score > alpha {
            alpha = score;
        }
    }

    alpha
}

pub(crate) fn evalute_cur_side(board: &Board) -> i32 {
    evalute(board, board.game_state.side_to_move)
}

pub(crate) fn calc_phase(board: &Board) -> i32 {
    let n = (board.get_bb(Side::White, Piece::Knight).count_ones()
        + board.get_bb(Side::Black, Piece::Knight).count_ones()) as i32;
    let b = (board.get_bb(Side::White, Piece::Bishop).count_ones()
        + board.get_bb(Side::Black, Piece::Bishop).count_ones()) as i32;
    let r = (board.get_bb(Side::White, Piece::Rook).count_ones()
        + board.get_bb(Side::Black, Piece::Rook).count_ones()) as i32;
    let q = (board.get_bb(Side::White, Piece::Queen).count_ones()
        + board.get_bb(Side::Black, Piece::Queen).count_ones()) as i32;

    let ph = n + b + 2 * r + 4 * q;
    ph.clamp(0, 24)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_function() {
        let board = Board::get_start_position();

        assert_eq!(0, evalute(&board, board.game_state.side_to_move));
    }
}
