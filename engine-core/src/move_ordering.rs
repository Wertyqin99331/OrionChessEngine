use crate::{
    board::Board,
    chess_consts,
    enums::{Move, Piece},
    searching::SearchState,
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

pub(crate) fn score_move(
    mv: Move,
    board: &Board,
    ply: u32,
    sort_quiet_moves: bool,
    use_see: bool,
    tt_move: Option<Move>,
    search_state: &SearchState,
) -> i32 {
    if let Some(tt_mv) = tt_move
        && mv == tt_mv
    {
        return 250_000;
    }

    if ply == 0
        && let Some(pv_mv) = search_state.prev_pv_first_move
        && pv_mv == mv
    {
        return 200_000;
    }

    if mv.is_capture() {
        let base_score =
            get_mvv_score(mv.get_moving_piece().unwrap(), mv.get_captured().unwrap()) as i32;

        if use_see {
            let see_score = board.see(mv);

            if see_score > 0 {
                return 100_000 + base_score + see_score;
            } else if see_score == 0 {
                return 50_000 + base_score + see_score;
            } else {
                return base_score - 100_000;
            }
        }

        base_score + 100_000
    } else {
        if !sort_quiet_moves {
            return 0;
        }

        let pl = ply as usize;

        if let Some(first_km) = search_state.killers[0][pl]
            && first_km == mv
        {
            return 90_000;
        } else if let Some(second_km) = search_state.killers[1][pl]
            && second_km == mv
        {
            return 80_000;
        } else {
            let (from, to) = mv.get_from_to();

            search_state.history[from.index() as usize][to.index() as usize] as i32
        }
    }
}

pub(crate) fn sort_moves(
    moves: &mut [Move],
    board: &Board,
    ply: u32,
    sort_quiet_moves: bool,
    use_see: bool,
    tt_move: Option<Move>,
    search_state: &SearchState,
) -> [i32; chess_consts::MOVES_BUF_SIZE] {
    let mut scores = [0i32; chess_consts::MOVES_BUF_SIZE];

    let n = moves.len();

    if n <= 1 {
        return scores;
    }

    for i in 0..n {
        scores[i] = score_move(
            moves[i],
            board,
            ply,
            sort_quiet_moves,
            use_see,
            tt_move,
            search_state,
        );
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

    scores
}
