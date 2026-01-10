use crate::{
    chess_consts,
    enums::{Move, Piece},
};

const MVV_TABLE: [[u32; chess_consts::PIECE_TYPES_COUNT]; chess_consts::PIECE_TYPES_COUNT] = [
    [105, 205, 305, 405, 505, 605],
    [104, 204, 304, 404, 504, 604],
    [103, 203, 303, 403, 503, 603],
    [102, 202, 302, 402, 502, 602],
    [101, 201, 301, 401, 501, 601],
    [100, 200, 300, 400, 500, 600],
];

const fn get_mvv_score(attacker: Piece, victim: Piece) -> u32 {
    MVV_TABLE[attacker.index() as usize][victim.index() as usize]
}

pub(crate) fn score_move(mv: Move) -> i32 {
    if mv.is_capture() {
        let (piece, captured) = match mv {
            Move::Normal {
                piece, captured, ..
            } => (piece, captured.unwrap()),
            _ => unreachable!(),
        };

        get_mvv_score(piece, captured) as i32
    } else {
        0
    }
}

pub(crate) fn sort_moves(moves: &mut [Move]) {
    moves.sort_by(|a, b| score_move(*b).cmp(&score_move(*a)));
}

#[cfg(test)]
mod tests {
    use crate::{enums::Side, fen_parser};

    use super::*;

    #[test]
    #[ignore]
    fn test_score_move_function() {
        let mut board =
            fen_parser::parse_fen_string("1k6/8/8/2q1r2P/3P4/B2N4/8/K7 b - - 0 1").unwrap();

        let mut moves = board.generate_all_legal_moves_to_vec(Side::White);

        sort_moves(&mut moves);

        for mv in moves {
            println!("Move: {mv:?}, score: {}", score_move(mv));
        }
    }
}
