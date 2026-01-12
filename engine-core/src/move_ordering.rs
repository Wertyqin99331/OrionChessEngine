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

pub(crate) static mut KILLER_MOVES: [[Option<Move>; chess_consts::MAX_PLY]; 2] =
    [[None; chess_consts::MAX_PLY]; 2];

#[allow(static_mut_refs)]
pub(crate) fn update_killers(mv: Move, ply: u32) {
    let p = ply as usize;

    unsafe {
        let km = &mut KILLER_MOVES;

        let k0 = km[0][p];

        if k0 == Some(mv) {
            return;
        }

        km[1][p] = k0;
        km[0][p] = Some(mv);
    }
}

#[allow(static_mut_refs)]
pub(crate) fn clear_killers() {
    unsafe { KILLER_MOVES.fill([None; chess_consts::MAX_PLY]) };
}

static mut HISTORY_MOVES: [[u64; chess_consts::SQUARES_COUNT]; chess_consts::SQUARES_COUNT] =
    [[0; chess_consts::SQUARES_COUNT]; chess_consts::SQUARES_COUNT];

pub(crate) fn update_history(mv: Move, depth: u32) {
    let (from, to) = mv.get_from_to();
    let f = from.index() as usize;
    let t = to.index() as usize;
    let add = (depth * depth) as u64;

    unsafe {
        HISTORY_MOVES[f][t] = HISTORY_MOVES[f][t].saturating_add(add);
    }
}

pub(crate) fn normalize_history() {
    unsafe {
        for from in 0..chess_consts::SQUARES_COUNT {
            for to in 0..chess_consts::SQUARES_COUNT {
                HISTORY_MOVES[from][to] >>= 1;
            }
        }
    }
}

pub(crate) fn score_move(mv: Move, ply: u32, only_captures: bool) -> i32 {
    if mv.is_capture() {
        let (piece, captured) = match mv {
            Move::Normal {
                piece, captured, ..
            } => (piece, captured.unwrap()),
            _ => unreachable!(),
        };

        get_mvv_score(piece, captured) as i32 + 100_000
    } else {
        if only_captures {
            return 0;
        }

        if let Some(first_km) = unsafe { KILLER_MOVES }[0][ply as usize]
            && first_km == mv
        {
            return 90_000;
        } else if let Some(second_km) = unsafe { KILLER_MOVES }[1][ply as usize]
            && second_km == mv
        {
            return 80_000;
        } else {
            let (from, to) = mv.get_from_to();

            (unsafe { HISTORY_MOVES })[from.index() as usize][to.index() as usize] as i32
        }
    }
}

pub(crate) fn sort_moves(moves: &mut [Move], ply: u32, only_captures: bool) {
    let n = moves.len();

    if n <= 1 {
        return;
    }

    let mut scores = [0i32; chess_consts::MOVES_BUF_SIZE];
    for i in 0..n {
        scores[i] = score_move(moves[i], ply, only_captures);
    }

    for i in 1..n {
        let mv = moves[i];
        let sc = scores[i];

        let mut j = i;

        while j > 0 && scores[j - 1] < sc {
            moves[j] = moves[j - 1];
            scores[j] = scores[j - 1];
            j -= 1;
        }

        moves[j] = mv;
        scores[j] = sc;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        enums::{MoveFlags, Side, Square},
        fen_parser,
    };

    use super::*;

    #[test]
    #[ignore]
    fn test_score_move_function() {
        let mut board =
            fen_parser::parse_fen_string("1k6/8/8/2q1r2P/3P4/B2N4/8/K7 b - - 0 1").unwrap();

        let mut moves = board.generate_all_legal_moves_to_vec(Side::White);

        sort_moves(&mut moves, 0, false);

        for mv in moves {
            println!("Move: {mv:?}, score: {}", score_move(mv, 0, false));
        }
    }

    #[test]
    #[ignore]
    fn test_normalize_history_function() {
        update_history(
            Move::Normal {
                from: Square::A1,
                to: Square::B1,
                piece: Piece::Queen,
                captured: None,
                promo: None,
                flags: MoveFlags::empty(),
            },
            5,
        );
        println!("{:?}", unsafe { HISTORY_MOVES });

        normalize_history();
        println!("{:?}", unsafe { HISTORY_MOVES });
    }
}
