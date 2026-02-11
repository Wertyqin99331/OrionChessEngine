use std::sync::atomic::Ordering;

use crate::{
    board::Board,
    chess_consts::{self},
    enums::{File, Piece, Rank, Side, Square},
    helpers::{self, get_attacks_mask},
    move_generator::MoveBuffer,
    move_ordering, pawn_attack_table,
    searching::{self, SearchState},
};

pub(crate) const MATE_EVALUATION: i32 = 30_000;
pub(crate) const MATE_SCORE: i32 = 29_000;
pub(crate) const MAX_PHASE: i32 = 24;

const PIECE_VALUES: [i32; chess_consts::PIECE_TYPES_COUNT] = [100, 300, 300, 500, 1_000, 10_000];

pub(crate) const fn get_piece_value(piece: Piece) -> i32 {
    PIECE_VALUES[piece.index_usize()]
}

pub(crate) fn eval_pos(board: &Board, side: Side) -> i32 {
    if is_unsufficient_material(board) {
        return 0;
    }

    let (mut mg, mut eg) = (0, 0);

    let mut add_scores = |scores: (i32, i32)| {
        mg += scores.0;
        eg += scores.1;
    };

    add_scores(eval_material_pst(board));
    add_scores(eval_pawns(board));
    add_scores(eval_rooks(board));
    add_scores(eval_mobility(board));
    add_scores(eval_king_safety(board));
    add_scores(eval_bishops(board));
    add_scores(eval_knights(board));

    let phase = calc_phase(board);
    let score = ((phase as i64 * mg as i64 + (MAX_PHASE - phase) as i64 * eg as i64)
        / MAX_PHASE as i64) as i32;
    return if side == Side::White { score } else { -score };
}

fn is_unsufficient_material(board: &Board) -> bool {
    let piece_candidates = [Piece::Pawn, Piece::Rook, Piece::Queen];

    for piece in piece_candidates {
        if board.get_bb(Side::White, piece).count_ones() > 0
            || board.get_bb(Side::Black, piece).count_ones() > 0
        {
            return false;
        }
    }

    let white_knights = board.get_bb(Side::White, Piece::Knight).count_ones();
    let white_bishops = board.get_bb(Side::White, Piece::Bishop).count_ones();
    let black_knights = board.get_bb(Side::Black, Piece::Knight).count_ones();
    let black_bishops = board.get_bb(Side::Black, Piece::Bishop).count_ones();
    let white_minor_pieces = white_knights + white_bishops;
    let black_minor_pieces = black_knights + black_bishops;

    if white_minor_pieces >= 2 || black_minor_pieces >= 2 {
        return false;
    }

    if white_minor_pieces == 0 && black_minor_pieces == 0 {
        return true;
    }

    if (white_minor_pieces == 0 && black_minor_pieces == 1)
        || (black_minor_pieces == 0 && white_minor_pieces == 1)
    {
        return true;
    }

    if white_bishops == 1 && black_bishops == 1 {
        let white_bishop_sq = helpers::get_squares_iter(board.get_bb(Side::White, Piece::Bishop))
            .next()
            .unwrap();
        let black_bishop_sq = helpers::get_squares_iter(board.get_bb(Side::Black, Piece::Bishop))
            .next()
            .unwrap();

        if white_bishop_sq.color() == black_bishop_sq.color() {
            return true;
        }
    }

    return false;
}

mod pst_tables {
    use crate::{
        chess_consts,
        enums::{Side, Square},
        evaluation::Phase,
    };

    type PstTable = [[i32; chess_consts::SQUARES_COUNT]; chess_consts::PHASES_COUNT];

    pub(super) fn get_pst_value(table: &PstTable, side: Side, sq: Square, phase: Phase) -> i32 {
        let index = if side == Side::White {
            sq.index() as usize ^ 56
        } else {
            sq.index() as usize
        };

        table[phase.index_usize()][index]
    }

    #[rustfmt::skip]
    pub(super) const PAWN_PST: PstTable = [
        // MIDGAME
        [
             0,   0,   0,   0,   0,   0,   0,   0,
            50,  50,  50,  50,  50,  50,  50,  50,
            10,  10,  20,  30,  30,  20,  10,  10,
             5,   5,  10,  25,  25,  10,   5,   5,
             0,   0,   0,  20,  20,   0,   0,   0,
             5,  -5, -10,   0,   0, -10,  -5,   5,
             5,  10,  10, -20, -20,  10,  10,   5,
             0,   0,   0,   0,   0,   0,   0,   0,
        ],
        // ENDGAME
        [
             0,   0,   0,   0,   0,   0,   0,   0,
            80,  80,  80,  80,  80,  80,  80,  80,
            50,  50,  50,  60,  60,  50,  50,  50,
            30,  30,  30,  40,  40,  30,  30,  30,
            20,  20,  20,  30,  30,  20,  20,  20,
            10,  10,  10,  20,  20,  10,  10,  10,
             0,   0,   0,   0,   0,   0,   0,   0,
             0,   0,   0,   0,   0,   0,   0,   0,
        ],
    ];

    #[rustfmt::skip]
    pub(super) const KNIGHT_PST: PstTable = [
        // MIDGAME
        [
           -50, -40, -30, -30, -30, -30, -40, -50,
           -40, -20,   0,   5,   5,   0, -20, -40,
           -30,   5,  10,  15,  15,  10,   5, -30,
           -30,   0,  15,  20,  20,  15,   0, -30,
           -30,   5,  15,  20,  20,  15,   5, -30,
           -30,   0,  10,  15,  15,  10,   0, -30,
           -40, -20,   0,   0,   0,   0, -20, -40,
           -50, -40, -30, -30, -30, -30, -40, -50,
        ],
        // ENDGAME
        [
           -40, -30, -20, -20, -20, -20, -30, -40,
           -30, -10,   0,   5,   5,   0, -10, -30,
           -20,   5,  10,  15,  15,  10,   5, -20,
           -20,   5,  15,  20,  20,  15,   5, -20,
           -20,   5,  15,  20,  20,  15,   5, -20,
           -20,   0,  10,  15,  15,  10,   0, -20,
           -30, -10,   0,   0,   0,   0, -10, -30,
           -40, -30, -20, -20, -20, -20, -30, -40,
        ],
    ];

    #[rustfmt::skip]
    pub(super) const BISHOP_PST: PstTable = [
        // MIDGAME
        [
            -20, -10, -10, -10, -10, -10, -10, -20,
            -10,   0,   0,   0,   0,   0,   0, -10,
            -10,   0,   5,  10,  10,   5,   0, -10,
            -10,   5,   5,  10,  10,   5,   5, -10,
            -10,   0,  10,  10,  10,  10,   0, -10,
            -10,  10,  10,  10,  10,  10,  10, -10,
            -10,   5,   0,   0,   0,   0,   5, -10,
            -20, -10, -10, -10, -10, -10, -10, -20,
             ],
        // ENDGAME
        [
           -10,  -5,  -5,  -5,  -5,  -5,  -5, -10,
            -5,  10,   5,   5,   5,   5,  10,  -5,
            -5,   5,  10,  15,  15,  10,   5,  -5,
            -5,   5,  15,  20,  20,  15,   5,  -5,
            -5,   5,  15,  20,  20,  15,   5,  -5,
            -5,   5,  10,  15,  15,  10,   5,  -5,
            -5,  10,   5,   5,   5,   5,  10,  -5,
           -10,  -5,  -5,  -5,  -5,  -5,  -5, -10,
        ],
    ];

    #[rustfmt::skip]
    pub(super) const ROOK_PST: PstTable = [
        // MIDGAME
        [
             0,   0,   0,   0,   0,   0,   0,   0,
             5,  10,  10,  10,  10,  10,  10,   5,
            -5,   0,   0,   0,   0,   0,   0,  -5,
            -5,   0,   0,   0,   0,   0,   0,  -5,
            -5,   0,   0,   0,   0,   0,   0,  -5,
            -5,   0,   0,   0,   0,   0,   0,  -5,
            -5,   0,   0,   0,   0,   0,   0,  -5,
             0,   0,   0,   5,   5,   0,   0,   0,
        ],
        // ENDGAME
        [
             0,   0,   5,  10,  10,   5,   0,   0,
             5,  10,  10,  15,  15,  10,  10,   5,
             0,   0,   5,  10,  10,   5,   0,   0,
             0,   0,   5,  10,  10,   5,   0,   0,
             0,   0,   5,  10,  10,   5,   0,   0,
             0,   0,   5,  10,  10,   5,   0,   0,
             0,   0,   5,  10,  10,   5,   0,   0,
             0,   0,   0,   5,   5,   0,   0,   0,
        ],
    ];

    #[rustfmt::skip]
    pub(super) const QUEEN_PST: PstTable = [
        // MIDGAME
        [
           -20, -10, -10,  -5,  -5, -10, -10, -20,
           -10,   0,   0,   0,   0,   0,   0, -10,
           -10,   0,   5,   5,   5,   5,   0, -10,
            -5,   0,   5,   5,   5,   5,   0,  -5,
            -5,   0,   5,   5,   5,   5,   0,  -5,
           -10,   0,   5,   5,   5,   5,   0, -10,
           -10,   0,   0,   0,   0,   0,   0, -10,
           -20, -10, -10,  -5,  -5, -10, -10, -20,
        ],
        // ENDGAME
        [
           -10,  -5,  -5,  -5,  -5,  -5,  -5, -10,
            -5,   0,   0,   0,   0,   0,   0,  -5,
            -5,   0,   5,   5,   5,   5,   0,  -5,
            -5,   0,   5,   5,   5,   5,   0,  -5,
            -5,   0,   5,   5,   5,   5,   0,  -5,
            -5,   0,   5,   5,   5,   5,   0,  -5,
            -5,   0,   0,   0,   0,   0,   0,  -5,
           -10,  -5,  -5,  -5,  -5,  -5,  -5, -10,
        ],
    ];

    #[rustfmt::skip]
    pub const KING_PST: PstTable = [
        // MIDGAME
        [
            -30, -40, -40, -50, -50, -40, -40, -30,
            -30, -40, -40, -50, -50, -40, -40, -30,
            -30, -40, -40, -50, -50, -40, -40, -30,
            -30, -40, -40, -50, -50, -40, -40, -30,
            -20, -30, -30, -40, -40, -30, -30, -20,
            -10, -20, -20, -20, -20, -20, -20, -10,
             20,  20,   0,   0,   0,   0,  20,  20,
             20,  30,  10,   0,   0,  10,  30,  20,
        ],

        // ENDGAME
        [
            -50, -30, -30, -30, -30, -30, -30, -50,
            -30, -30,   0,   0,   0,   0, -30, -30,
            -30, -10,  20,  30,  30,  20, -10, -30,
            -30, -10,  30,  40,  40,  30, -10, -30,
            -30, -10,  30,  40,  40,  30, -10, -30,
            -30, -10,  20,  30,  30,  20, -10, -30,
            -30, -20, -10,   0,   0, -10, -20, -30,
            -50, -40, -30, -20, -20, -30, -40, -50,
        ],
    ];
}

mod piece_scores {

    use crate::{enums::Piece, evaluation::Phase};

    pub(super) const PAWN_SCORES: [i32; 2] = [100, 125];
    pub(super) const KNIGHT_SCORES: [i32; 2] = [315, 305];
    pub(super) const BISHOP_SCORES: [i32; 2] = [325, 330];
    pub(super) const ROOK_SCORES: [i32; 2] = [500, 520];
    pub(super) const QUEEN_SCORES: [i32; 2] = [920, 920];
    pub(super) const KING_SCORES: [i32; 2] = [10_000, 10_000];

    pub(super) fn get_piece_score(piece: Piece, phase: Phase) -> i32 {
        let phase_index = phase.index_usize();

        match piece {
            Piece::Pawn => PAWN_SCORES[phase_index],
            Piece::Knight => KNIGHT_SCORES[phase_index],
            Piece::Bishop => BISHOP_SCORES[phase_index],
            Piece::Rook => ROOK_SCORES[phase_index],
            Piece::Queen => QUEEN_SCORES[phase_index],
            Piece::King => KING_SCORES[phase_index],
        }
    }
}

fn eval_material_pst(board: &Board) -> (i32, i32) {
    let mut mg = 0;
    let mut eg = 0;

    for piece in Piece::all() {
        let white_bb = board.get_bb(Side::White, piece);
        let black_bb = board.get_bb(Side::Black, piece);

        mg += white_bb.count_ones() as i32 * piece_scores::get_piece_score(piece, Phase::Midgame);
        eg += white_bb.count_ones() as i32 * piece_scores::get_piece_score(piece, Phase::Endgame);
        mg -= black_bb.count_ones() as i32 * piece_scores::get_piece_score(piece, Phase::Midgame);
        eg -= black_bb.count_ones() as i32 * piece_scores::get_piece_score(piece, Phase::Endgame);

        let pst_table = match piece {
            Piece::Pawn => pst_tables::PAWN_PST,
            Piece::Knight => pst_tables::KNIGHT_PST,
            Piece::Bishop => pst_tables::BISHOP_PST,
            Piece::Rook => pst_tables::ROOK_PST,
            Piece::Queen => pst_tables::QUEEN_PST,
            Piece::King => pst_tables::KING_PST,
        };

        for sq in helpers::get_squares_iter(white_bb) {
            mg += pst_tables::get_pst_value(&pst_table, Side::White, sq, Phase::Midgame);
            eg += pst_tables::get_pst_value(&pst_table, Side::White, sq, Phase::Endgame);
        }

        for sq in helpers::get_squares_iter(black_bb) {
            mg -= pst_tables::get_pst_value(&pst_table, Side::Black, sq, Phase::Midgame);
            eg -= pst_tables::get_pst_value(&pst_table, Side::Black, sq, Phase::Endgame);
        }
    }

    (mg, eg)
}

const ISOLATED_MASKS: [u64; chess_consts::BOARD_SIZE] = const {
    let mut masks = [0; chess_consts::BOARD_SIZE];

    let mut file = 0;

    while file < chess_consts::BOARD_SIZE {
        let mut mask = 0;

        if file != 0 {
            mask |= chess_consts::FILE_MASKS[file - 1];
        }

        if file != chess_consts::BOARD_SIZE - 1 {
            mask |= chess_consts::FILE_MASKS[file + 1]
        }

        masks[file] = mask;

        file += 1;
    }

    masks
};

const fn get_isolated_mask(file: File) -> u64 {
    ISOLATED_MASKS[file.index_usize()]
}

const PASSED_MASKS: [[u64; chess_consts::SQUARES_COUNT]; chess_consts::SIDES_COUNT] = const {
    let mut masks = [[0; chess_consts::SQUARES_COUNT]; chess_consts::SIDES_COUNT];
    let mut sq = 0;

    while sq < chess_consts::SQUARES_COUNT {
        let square = unsafe { Square::from_u8_unchecked(sq as u8) };

        let mut white_mask = 0;
        white_mask |= get_isolated_mask(square.file());
        white_mask |= chess_consts::get_file_mask(square.file());
        let mut white_rank = square.rank().index_usize() as isize;
        while white_rank >= 0 {
            white_mask &= !chess_consts::RANK_MASKS[white_rank as usize];
            white_rank -= 1;
        }
        masks[Side::White.index_usize()][sq] = white_mask;

        let mut black_mask = 0;
        black_mask |= get_isolated_mask(square.file());
        black_mask |= chess_consts::get_file_mask(square.file());
        let mut black_rank = square.rank().index_usize() as isize;
        while black_rank < chess_consts::BOARD_SIZE as isize {
            black_mask &= !chess_consts::RANK_MASKS[black_rank as usize];
            black_rank += 1;
        }
        masks[Side::Black.index_usize()][sq] = black_mask;
        sq += 1;
    }
    masks
};

const fn get_passed_mask(side: Side, sq: Square) -> u64 {
    PASSED_MASKS[side.index_usize()][sq.index_usize()]
}

const DOUBLE_PAWN_PENALTY: [i32; chess_consts::PHASES_COUNT] = [15, 10];
const ISOLATED_PAWN_PENALTY: [i32; chess_consts::PHASES_COUNT] = [20, 12];
const PASSED_PAWN_BONUSES: [[i32; chess_consts::BOARD_SIZE]; chess_consts::PHASES_COUNT] = [
    [0, 5, 10, 20, 30, 40, 50, 0],
    [0, 10, 30, 60, 90, 130, 160, 0],
];
const PASSED_PAWN_BLOCKADE_BY_KING_BONUS_MULTIPLIER: f32 = 0.3;
const PASSED_PAWN_BLOCKADE_BY_MINOR_BONUS_MULTIPLIER: f32 = 0.5;

fn eval_pawns(board: &Board) -> (i32, i32) {
    let mut mg = 0;
    let mut eg = 0;

    let white_pawns_bb = board.get_bb(Side::White, Piece::Pawn);
    let black_pawns_bb = board.get_bb(Side::Black, Piece::Pawn);

    for file in File::all() {
        let file_mask = chess_consts::get_file_mask(file);

        let white_pawns_count = (white_pawns_bb & file_mask).count_ones() as i32;
        if white_pawns_count >= 2 {
            mg -= (white_pawns_count - 1) * DOUBLE_PAWN_PENALTY[Phase::Midgame.index_usize()];
            eg -= (white_pawns_count - 1) * DOUBLE_PAWN_PENALTY[Phase::Endgame.index_usize()];
        }

        let black_pawns_count = (black_pawns_bb & file_mask).count_ones() as i32;
        if black_pawns_count >= 2 {
            mg += (black_pawns_count - 1) * DOUBLE_PAWN_PENALTY[Phase::Midgame.index_usize()];
            eg += (black_pawns_count - 1) * DOUBLE_PAWN_PENALTY[Phase::Endgame.index_usize()];
        }
    }

    let mut eval_isolated_and_passed_pawns = |side: Side| {
        let (mut local_mg, mut local_eg) = (0, 0);

        let our_pawn_bb = board.get_bb(side, Piece::Pawn);
        let enemy_pawns_bb = board.get_bb(side.opposite(), Piece::Pawn);

        for sq in helpers::get_squares_iter(our_pawn_bb) {
            let isolated_mask = get_isolated_mask(sq.file());
            if isolated_mask & our_pawn_bb == 0 {
                local_mg -= ISOLATED_PAWN_PENALTY[MIDGAME_INDEX];
                local_eg -= ISOLATED_PAWN_PENALTY[ENDGAME_INDEX];
            }

            let passed_mask = get_passed_mask(side, sq);
            let is_passed = passed_mask & enemy_pawns_bb == 0;
            if is_passed {
                let (base_mg_bonus, base_eg_bonus) = match side {
                    Side::White => (
                        PASSED_PAWN_BONUSES[MIDGAME_INDEX][sq.rank().index_usize()],
                        PASSED_PAWN_BONUSES[ENDGAME_INDEX][sq.rank().index_usize()],
                    ),
                    Side::Black => (
                        PASSED_PAWN_BONUSES[MIDGAME_INDEX][7 - sq.rank().index_usize()],
                        PASSED_PAWN_BONUSES[ENDGAME_INDEX][7 - sq.rank().index_usize()],
                    ),
                };

                let (mut final_mg_bonus, mut final_eg_bonus) = (base_mg_bonus, base_eg_bonus);

                let blockade_square = sq.forward(side);
                if board.is_square_occupied_by_side(side.opposite(), blockade_square) {
                    let occupant = board
                        .get_piece_type_on_square(side.opposite(), blockade_square)
                        .unwrap();

                    let multiplier = match occupant {
                        Piece::King => PASSED_PAWN_BLOCKADE_BY_KING_BONUS_MULTIPLIER,
                        Piece::Knight | Piece::Bishop => {
                            PASSED_PAWN_BLOCKADE_BY_MINOR_BONUS_MULTIPLIER
                        }
                        _ => 1.0,
                    };

                    final_mg_bonus = (final_mg_bonus as f32 * multiplier) as i32;
                    final_eg_bonus = (final_eg_bonus as f32 * multiplier) as i32;
                }

                local_mg += final_mg_bonus;
                local_eg += final_eg_bonus;
            } else {
                // TODO passing candidates
            }
        }

        match side {
            Side::White => {
                mg += local_mg;
                eg += local_eg;
            }
            Side::Black => {
                mg -= local_mg;
                eg -= local_eg;
            }
        }
    };

    eval_isolated_and_passed_pawns(Side::White);
    eval_isolated_and_passed_pawns(Side::Black);

    (mg, eg)
}

const SEMI_OPEN_FILE_SCORE: [i32; chess_consts::PHASES_COUNT] = [15, 12];
const OPEN_FILE_SCORE: [i32; chess_consts::PHASES_COUNT] = [25, 20];
const ROOK_ON_BACK_RANK_BONUS: [i32; chess_consts::PHASES_COUNT] = [25, 15];
const ROOK_HORIZONTAL_PAWN_ATTACK_BONUS: [i32; chess_consts::PHASES_COUNT] = [10, 15];

fn eval_rooks(board: &Board) -> (i32, i32) {
    let (mut mg, mut eg) = (0, 0);

    let all_pawns_bb =
        board.get_bb(Side::White, Piece::Pawn) | board.get_bb(Side::Black, Piece::Pawn);
    let mg_index = Phase::Midgame.index_usize();
    let eg_index = Phase::Endgame.index_usize();

    let mut eval_rooks = |side: Side| {
        let (mut local_mg, mut local_eg) = (0, 0);

        let our_pawns_bb = board.get_bb(side, Piece::Pawn);
        let enemy_pawns_bb = board.get_bb(side.opposite(), Piece::Pawn);
        let back_rank = if side == Side::White {
            Rank::R7
        } else {
            Rank::R2
        };

        for sq in helpers::get_squares_iter(board.get_bb(side, Piece::Rook)) {
            let file_mask = chess_consts::get_file_mask(sq.file());

            if file_mask & all_pawns_bb == 0 {
                local_mg += OPEN_FILE_SCORE[mg_index];
                local_eg += OPEN_FILE_SCORE[eg_index];
            } else if file_mask & our_pawns_bb == 0 {
                local_mg += SEMI_OPEN_FILE_SCORE[mg_index];
                local_eg += SEMI_OPEN_FILE_SCORE[eg_index];
            }

            if sq.rank() == back_rank {
                local_mg += ROOK_ON_BACK_RANK_BONUS[mg_index];
                local_eg += ROOK_ON_BACK_RANK_BONUS[eg_index];
            }

            let rank_mask = chess_consts::get_rank_mask(sq.rank());
            let pawn_rank_targets = (enemy_pawns_bb & rank_mask).count_ones() as i32;

            if pawn_rank_targets > 0 {
                local_mg += pawn_rank_targets * ROOK_HORIZONTAL_PAWN_ATTACK_BONUS[mg_index];
                local_eg += pawn_rank_targets * ROOK_HORIZONTAL_PAWN_ATTACK_BONUS[eg_index];
            }
        }

        match side {
            Side::White => {
                mg += local_mg;
                eg += local_eg;
            }
            Side::Black => {
                mg -= local_mg;
                eg -= local_eg;
            }
        }
    };
    eval_rooks(Side::White);
    eval_rooks(Side::Black);

    (mg, eg)
}

#[rustfmt::skip]
const KNIGHT_MOBILITY: [[i32; 9]; 2] = [
    // MG
    [-50, -30, -10, 0, 5, 10, 15, 20, 25],
    // EG (slightly more aggressive in endgame)
    [-40, -25, -5, 0, 10, 15, 20, 25, 30],
];

#[rustfmt::skip]
const BISHOP_MOBILITY: [[i32; 14]; 2] = [
    // MG
    [-40, -25, -10, 0, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50],
    // EG
    [-30, -20, -5, 0, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55],
];

#[rustfmt::skip]
const ROOK_MOBILITY: [[i32; 15]; 2] = [
    // MG
    [-30, -15, -5, 0, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55],
    // EG
    [-20, -10, 0, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55, 60],
];

#[rustfmt::skip]
const QUEEN_MOBILITY: [[i32; 28]; 2] = [
    // MG
    [-20, -10, -5, 0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30, 32, 34, 36, 38, 40,
     42, 44, 46, 48],
    // EG
    [-15, -5, 0, 5, 7, 10, 12, 15, 18, 20, 22, 25, 28, 30, 32, 35, 38, 40, 42, 45, 48, 50, 52, 55,
     58, 60, 62, 65],
];

fn eval_mobility(board: &Board) -> (i32, i32) {
    let (mut mg, mut eg) = (0, 0);

    let white_pawn_attacks = board.generate_pawn_attacks_bb(Side::White);
    let black_pawn_attacks = board.generate_pawn_attacks_bb(Side::Black);
    let not_white_pawn_attacks = !white_pawn_attacks;
    let not_black_pawn_attacks = !black_pawn_attacks;
    let not_white_bb = !board.get_occupancy_bb(Side::White);
    let not_black_bb = !board.get_occupancy_bb(Side::Black);

    let mg_index = Phase::Midgame.index_usize();
    let eg_index = Phase::Endgame.index_usize();
    let mobility_pieces = [
        (
            Piece::Knight,
            &KNIGHT_MOBILITY[mg_index].as_slice(),
            &KNIGHT_MOBILITY[eg_index].as_slice(),
        ), // mg, eg
        (
            Piece::Bishop,
            &BISHOP_MOBILITY[mg_index].as_slice(),
            &BISHOP_MOBILITY[eg_index].as_slice(),
        ),
        (
            Piece::Rook,
            &ROOK_MOBILITY[mg_index].as_slice(),
            &ROOK_MOBILITY[eg_index].as_slice(),
        ),
        (
            Piece::Queen,
            &QUEEN_MOBILITY[mg_index].as_slice(),
            &QUEEN_MOBILITY[eg_index].as_slice(),
        ),
    ];

    for (piece, mg_scores, eg_scores) in mobility_pieces {
        for sq in helpers::get_squares_iter(board.get_bb(Side::White, piece)) {
            let mut attacks_mask = get_attacks_mask(Side::White, piece, sq, board.global_occupancy);
            attacks_mask &= not_white_bb;
            attacks_mask &= not_black_pawn_attacks;

            mg += mg_scores[(attacks_mask.count_ones() as usize).min(mg_scores.len() - 1)];
            eg += eg_scores[(attacks_mask.count_ones() as usize).min(eg_scores.len() - 1)];
        }

        for sq in helpers::get_squares_iter(board.get_bb(Side::Black, piece)) {
            let mut attacks_mask = get_attacks_mask(Side::Black, piece, sq, board.global_occupancy);
            attacks_mask &= not_black_bb;
            attacks_mask &= not_white_pawn_attacks;

            mg -= mg_scores[(attacks_mask.count_ones() as usize).min(mg_scores.len() - 1)];
            eg -= eg_scores[(attacks_mask.count_ones() as usize).min(eg_scores.len() - 1)];
        }
    }

    (mg, eg)
}

const KING_SHIELD_MASKS: [[u64; chess_consts::SQUARES_COUNT]; chess_consts::SIDES_COUNT] = {
    let mut masks = [[0; chess_consts::SQUARES_COUNT]; chess_consts::SIDES_COUNT];

    let mut sq = 0;

    while sq < chess_consts::SQUARES_COUNT as u8 {
        let square_bb = (unsafe { Square::from_u8_unchecked(sq) }).get_bb();

        let mut white_mask = 0u64;
        white_mask |=
            (square_bb & chess_consts::NOT_A_FILE_BB & chess_consts::NOT_EIGHTH_RANK_BB) << 7;
        white_mask |= (square_bb & chess_consts::NOT_EIGHTH_RANK_BB) << 8;
        white_mask |=
            (square_bb & chess_consts::NOT_H_FILE_BB & chess_consts::NOT_EIGHTH_RANK_BB) << 9;
        masks[Side::White.index_usize()][sq as usize] = white_mask;

        let mut black_mask = 0u64;
        black_mask |=
            (square_bb & chess_consts::NOT_A_FILE_BB & chess_consts::NOT_FIRST_RANK_BB) >> 9;
        black_mask |= (square_bb & chess_consts::NOT_FIRST_RANK_BB) >> 8;
        black_mask |=
            (square_bb & chess_consts::NOT_H_FILE_BB & chess_consts::NOT_FIRST_RANK_BB) >> 7;
        masks[Side::Black.index_usize()][sq as usize] = black_mask;

        sq += 1;
    }

    masks
};

const fn get_king_shield_mask(side: Side, sq: Square) -> u64 {
    KING_SHIELD_MASKS[side.index_usize()][sq.index_usize()]
}

const KING_SHIELD_BONUS: [i32; 2] = [12, 2];
const KING_FILE_OPEN_PENALTY: [i32; 2] = [30, 5];
const ADJACENT_KING_FILE_OPEN_PENALTY: [i32; 2] = [15, 2];
const KING_FILE_SEMIOPEN_PENALTY: [i32; 2] = [20, 3];
const ADJACENT_KING_FILE_SEMIOPEN_PENALTY: [i32; 2] = [10, 1];

fn eval_king_safety(board: &Board) -> (i32, i32) {
    let (mut mg, mut eg) = (0, 0);

    let all_pawns_bb =
        board.get_bb(Side::White, Piece::Pawn) | board.get_bb(Side::Black, Piece::Pawn);

    let mut eval_king_safety = |side: Side| {
        let king_sq = board.get_king_square(side);
        let king_shield_bb = get_king_shield_mask(side, king_sq);
        let our_pawns_bb = board.get_bb(side, Piece::Pawn);

        let midgame_index = Phase::Midgame.index_usize();
        let endgame_index = Phase::Endgame.index_usize();

        // King shield pawns
        let shield_pawns_count = (our_pawns_bb & king_shield_bb).count_ones() as i32;
        match side {
            Side::White => {
                mg += shield_pawns_count * KING_SHIELD_BONUS[midgame_index];
                eg += shield_pawns_count * KING_SHIELD_BONUS[endgame_index];
            }
            Side::Black => {
                mg -= shield_pawns_count * KING_SHIELD_BONUS[midgame_index];
                eg -= shield_pawns_count * KING_SHIELD_BONUS[endgame_index];
            }
        }

        let king_file = king_sq.file();
        let king_file_mask = chess_consts::get_file_mask(king_file);

        // King file open/semi-open penalties
        if king_file_mask & all_pawns_bb == 0 {
            match side {
                Side::White => {
                    mg -= KING_FILE_OPEN_PENALTY[midgame_index];
                    eg -= KING_FILE_OPEN_PENALTY[endgame_index];
                }
                Side::Black => {
                    mg += KING_FILE_OPEN_PENALTY[midgame_index];
                    eg += KING_FILE_OPEN_PENALTY[endgame_index];
                }
            }
        } else if king_file_mask & our_pawns_bb == 0 {
            match side {
                Side::White => {
                    mg -= KING_FILE_SEMIOPEN_PENALTY[midgame_index];
                    eg -= KING_FILE_SEMIOPEN_PENALTY[endgame_index];
                }
                Side::Black => {
                    mg += KING_FILE_SEMIOPEN_PENALTY[midgame_index];
                    eg += KING_FILE_SEMIOPEN_PENALTY[endgame_index];
                }
            }
        }

        // Left file
        if king_file != File::A {
            let left_file = unsafe { File::from_u8_unchecked(king_file.index() - 1) };
            let left_file_mask = chess_consts::get_file_mask(left_file);

            if left_file_mask & all_pawns_bb == 0 {
                match side {
                    Side::White => {
                        mg -= ADJACENT_KING_FILE_OPEN_PENALTY[midgame_index];
                        eg -= ADJACENT_KING_FILE_OPEN_PENALTY[endgame_index];
                    }
                    Side::Black => {
                        mg += ADJACENT_KING_FILE_OPEN_PENALTY[midgame_index];
                        eg += ADJACENT_KING_FILE_OPEN_PENALTY[endgame_index];
                    }
                }
            } else if left_file_mask & our_pawns_bb == 0 {
                match side {
                    Side::White => {
                        mg -= ADJACENT_KING_FILE_SEMIOPEN_PENALTY[midgame_index];
                        eg -= ADJACENT_KING_FILE_SEMIOPEN_PENALTY[endgame_index];
                    }
                    Side::Black => {
                        mg += ADJACENT_KING_FILE_SEMIOPEN_PENALTY[midgame_index];
                        eg += ADJACENT_KING_FILE_SEMIOPEN_PENALTY[endgame_index];
                    }
                }
            }
        }

        // Right file
        if king_file != File::H {
            let right_file = unsafe { File::from_u8_unchecked(king_file.index() + 1) };
            let right_file_mask = chess_consts::get_file_mask(right_file);

            if right_file_mask & all_pawns_bb == 0 {
                match side {
                    Side::White => {
                        mg -= ADJACENT_KING_FILE_OPEN_PENALTY[midgame_index];
                        eg -= ADJACENT_KING_FILE_OPEN_PENALTY[endgame_index];
                    }
                    Side::Black => {
                        mg += ADJACENT_KING_FILE_OPEN_PENALTY[midgame_index];
                        eg += ADJACENT_KING_FILE_OPEN_PENALTY[endgame_index];
                    }
                }
            } else if right_file_mask & our_pawns_bb == 0 {
                match side {
                    Side::White => {
                        mg -= ADJACENT_KING_FILE_SEMIOPEN_PENALTY[midgame_index];
                        eg -= ADJACENT_KING_FILE_SEMIOPEN_PENALTY[endgame_index];
                    }
                    Side::Black => {
                        mg += ADJACENT_KING_FILE_SEMIOPEN_PENALTY[midgame_index];
                        eg += ADJACENT_KING_FILE_SEMIOPEN_PENALTY[endgame_index];
                    }
                }
            }
        }
    };

    eval_king_safety(Side::White);
    eval_king_safety(Side::Black);

    (mg, eg)
}

const BISHOP_PAIR_BONUS: [i32; 2] = [30, 50];

fn eval_bishops(board: &Board) -> (i32, i32) {
    let (mut mg, mut eg) = (0, 0);
    let mg_index = Phase::Midgame.index_usize();
    let eg_index = Phase::Endgame.index_usize();

    if board.get_bb(Side::White, Piece::Bishop).count_ones() >= 2 {
        mg += BISHOP_PAIR_BONUS[mg_index];
        eg += BISHOP_PAIR_BONUS[eg_index];
    }

    if board.get_bb(Side::Black, Piece::Bishop).count_ones() >= 2 {
        mg -= BISHOP_PAIR_BONUS[mg_index];
        eg -= BISHOP_PAIR_BONUS[eg_index];
    }

    (mg, eg)
}

const KNIGHT_OUTPOST_BONUS: [i32; 2] = [50, 30];

fn eval_knights(board: &Board) -> (i32, i32) {
    let (mut mg, mut eg) = (0, 0);

    let mut eval_knights_for_side = |side: Side| {
        let (mut local_mg, mut local_eg) = (0, 0);

        let outpost_ranks = match side {
            Side::White => [Rank::R4, Rank::R5, Rank::R6],
            Side::Black => [Rank::R5, Rank::R4, Rank::R3],
        };

        for sq in helpers::get_squares_iter(board.get_bb(side, Piece::Knight)) {
            if outpost_ranks.contains(&sq.rank()) {
                let support_pawns_mask =
                    pawn_attack_table::get_pawn_attacks_mask(side.opposite(), sq);
                if board.get_bb(side, Piece::Pawn) & support_pawns_mask != 0 {
                    let potential_pawn_attackers_mask = get_passed_mask(side, sq);

                    if potential_pawn_attackers_mask & board.get_bb(side.opposite(), Piece::Pawn)
                        == 0
                    {
                        local_mg += KNIGHT_OUTPOST_BONUS[MIDGAME_INDEX];
                        local_eg += KNIGHT_OUTPOST_BONUS[ENDGAME_INDEX];
                    }
                }
            }
        }

        match side {
            Side::White => {
                mg += local_mg;
                eg += local_eg;
            }
            Side::Black => {
                mg -= local_mg;
                eg -= local_eg;
            }
        }
    };

    eval_knights_for_side(Side::White);
    eval_knights_for_side(Side::Black);

    (mg, eg)
}

const DELTA_PRUNING_MARGIN: i32 = 100;

pub(crate) fn q_search(
    board: &mut Board,
    mut alpha: i32,
    beta: i32,
    bufs: &mut [MoveBuffer],
    ply: u32,
    search_state: &SearchState,
) -> i32 {
    searching::NODES_COUNTER.fetch_add(1, Ordering::Relaxed);

    let moving_side = board.game_state.side_to_move;

    let (cur_buf, rest_bufs) = bufs.split_first_mut().unwrap();
    cur_buf.clear();

    if board.is_in_check(moving_side) {
        board.generate_all_legal_moves(moving_side, cur_buf);

        if cur_buf.is_empty() {
            return -MATE_EVALUATION + ply as i32;
        }

        move_ordering::sort_moves(cur_buf, &board, ply, false, false, None, search_state);

        for mv in cur_buf.iter().copied() {
            board.make_move(mv);
            let score = -q_search(board, -beta, -alpha, rest_bufs, ply + 1, search_state);
            board.unmake_move();

            if score >= beta {
                return beta;
            }

            if score > alpha {
                alpha = score;
            }
        }

        return alpha;
    }

    let standing_pat = evalute_cur_side(&*board);

    if standing_pat >= beta {
        return beta;
    }

    if standing_pat > alpha {
        alpha = standing_pat;
    }

    board.generate_legal_captures(moving_side, cur_buf);
    move_ordering::sort_moves(cur_buf, &*board, ply, false, false, None, search_state);

    for capture_mv in cur_buf.iter().copied() {
        // if board.see(capture_mv) < 0 {
        //     continue;
        // }
        let mut gain = get_piece_value(capture_mv.get_captured().unwrap());

        if capture_mv.is_promo() {
            gain +=
                get_piece_value(capture_mv.get_promoted().unwrap()) - get_piece_value(Piece::Pawn);
        }

        if standing_pat + gain + DELTA_PRUNING_MARGIN < alpha {
            continue;
        }

        board.make_move(capture_mv);
        let score = -q_search(board, -beta, -alpha, rest_bufs, ply + 1, search_state);
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
    eval_pos(board, board.game_state.side_to_move)
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
    ph.clamp(0, MAX_PHASE)
}

#[repr(u8)]
#[derive(Clone, Copy)]
enum Phase {
    Midgame = 0,
    Endgame = 1,
}
const MIDGAME_INDEX: usize = 0;
const ENDGAME_INDEX: usize = 1;

impl Phase {
    fn index_usize(self) -> usize {
        self as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn print_isolated_masks() {
        for file in File::all() {
            println!("File {:?}", file);
            helpers::print_bitboard(ISOLATED_MASKS[file.index_usize()]);
        }
    }

    #[test]
    #[ignore]
    fn print_passed_masks() {
        for sq in Square::all() {
            println!("Square {:?}", sq);
            helpers::print_bitboard(PASSED_MASKS[Side::White.index_usize()][sq.index_usize()]);
            helpers::print_bitboard(PASSED_MASKS[Side::Black.index_usize()][sq.index_usize()]);
            println!();
        }
    }
}
