use std::sync::OnceLock;

use crate::{
    board::{Board, CastlingState},
    chess_consts::{self},
    enums::{File, Piece, Side, Square},
    pawn_attack_table,
    random_generator::XorShift64Star,
};

static PIECE_KEYS: OnceLock<
    [[[u64; chess_consts::SQUARES_COUNT]; chess_consts::PIECE_TYPES_COUNT];
        chess_consts::SIDES_COUNT],
> = OnceLock::new();

static SIDE_KEY: OnceLock<u64> = OnceLock::new();

static CASTLING_KEYS: OnceLock<[u64; 16]> = OnceLock::new();

static ENPASSANT_KEYS: OnceLock<[u64; chess_consts::BOARD_SIZE]> = OnceLock::new();

pub(crate) fn init_zobrist_hash_keys() {
    let mut rng_generator = XorShift64Star::new();

    let mut piece_keys = [[[0u64; chess_consts::SQUARES_COUNT]; chess_consts::PIECE_TYPES_COUNT];
        chess_consts::SIDES_COUNT];

    for side in Side::all() {
        for piece in Piece::all() {
            for sq in Square::all() {
                piece_keys[side.index_usize()][piece.index_usize()][sq.index_usize()] =
                    rng_generator.next_u64();
            }
        }
    }

    PIECE_KEYS.set(piece_keys).ok();

    let mut enpassant_keys = [0u64; chess_consts::BOARD_SIZE];

    for file in File::all() {
        enpassant_keys[file.index() as usize] = rng_generator.next_u64();
    }

    ENPASSANT_KEYS.set(enpassant_keys).ok();

    let mut castling_keys = [0u64; 16];

    for castling in 0..16 {
        castling_keys[castling] = rng_generator.next_u64();
    }

    CASTLING_KEYS.set(castling_keys).ok();

    SIDE_KEY.set(rng_generator.next_u64()).ok();
}

const UNINIT_ZOBRIST_ERROR: &'static str = "Zobrist  keys must be initialized before using";

pub(crate) fn get_piece_key(side: Side, piece: Piece, square: Square) -> u64 {
    PIECE_KEYS.get().expect(UNINIT_ZOBRIST_ERROR)[side.index_usize()][piece.index_usize()]
        [square.index_usize()]
}

pub(crate) fn get_enpassant_key(file: File) -> u64 {
    ENPASSANT_KEYS.get().expect(UNINIT_ZOBRIST_ERROR)[file.index_usize()]
}

pub(crate) fn get_castling_key(cs: CastlingState) -> u64 {
    CASTLING_KEYS.get().expect(UNINIT_ZOBRIST_ERROR)[cs.bits() as usize]
}

pub(crate) fn get_side_key() -> u64 {
    *SIDE_KEY.get().expect(UNINIT_ZOBRIST_ERROR)
}

pub(crate) fn need_to_hash_enpassant(board: &Board) -> bool {
    if let Some(enpassant_sq) = board.game_state.en_passant_square {
        let attacks_bb = pawn_attack_table::get_pawn_attacks_mask(
            board.game_state.side_to_move.opposite(),
            enpassant_sq,
        );

        if attacks_bb & board.get_bb(board.game_state.side_to_move, Piece::Pawn) != 0 {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use crate::{board::Board, enums::Side, fen_parser, init};

    #[test]
    fn test_zobrist_initialization() {
        init::init_engine();

        let board = Board::get_start_position();

        println!("Zobrist key is {}", board.game_state.zobrist_key);
        assert_ne!(board.game_state.zobrist_key, 0);
    }

    #[test]
    fn test_zobrist_side_hashing() {
        init::init_engine();

        let white = Board::get_start_position();

        let black = fen_parser::parse_fen_string(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
        )
        .unwrap();

        assert_ne!(white.game_state.zobrist_key, black.game_state.zobrist_key);
    }

    #[test]
    fn test_make_unmake_identical_zobrist() {
        init::init_engine();

        let mut board = Board::get_start_position();
        let moves = board.generate_all_legal_moves_to_vec(Side::White);

        let before_key = board.game_state.zobrist_key;

        board.make_move(moves[0]);

        board.unmake_move();
        let after_key = board.game_state.zobrist_key;

        assert_eq!(before_key, after_key);
    }
}
