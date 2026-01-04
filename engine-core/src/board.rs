use std::fmt::Display;

use crate::{
    chess_consts,
    enums::{Castling, Move, Piece, Side, Square},
    fen_parser, helpers,
    king_attack_table::get_king_attacks_mask,
    knight_attack_table::get_knight_attacks_mask,
    pawn_attack_table::get_pawn_attacks_mask,
    sliding_piece_attack_table::{
        get_bishop_attacks_mask, get_queen_attacks_mask, get_rook_attacks_mask,
    },
};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Board {
    pub(crate) bitboards: [u64; chess_consts::PIECE_TYPES_COUNT * 2],
    pub(crate) side_occupancies: [u64; chess_consts::SIDES_COUNT],
    pub(crate) global_occupancy: u64,
    pub(crate) game_state: GameState,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GameState {
    pub(crate) side_to_move: Side,
    pub(crate) en_passant_square: Option<Square>,
    pub(crate) castling_state: CastlingState,
    pub(crate) half_move_clock: u8,
    pub(crate) full_moves_count: u16,
}

impl Board {
    pub(crate) fn get_bb(&self, side: Side, piece: Piece) -> u64 {
        self.bitboards
            [(side.index() * chess_consts::PIECE_TYPES_COUNT as u8 + piece.index()) as usize]
    }

    pub(crate) fn get_bb_mut(&mut self, side: Side, piece: Piece) -> &mut u64 {
        &mut self.bitboards
            [(side.index() * chess_consts::PIECE_TYPES_COUNT as u8 + piece.index()) as usize]
    }

    pub(crate) fn get_occupancy_bb(&self, side: Side) -> u64 {
        self.side_occupancies[side.index() as usize]
    }

    pub(crate) fn recalc_occupancies(&mut self) {
        let mut white_occupancy_bb = chess_consts::EMPTY_BB;
        let mut black_occupancy_bb = chess_consts::EMPTY_BB;

        for piece in Piece::all() {
            white_occupancy_bb |= self.get_bb(Side::White, piece);
            black_occupancy_bb |= self.get_bb(Side::Black, piece);
        }

        self.side_occupancies[Side::White.index() as usize] = white_occupancy_bb;
        self.side_occupancies[Side::Black.index() as usize] = black_occupancy_bb;

        self.global_occupancy = white_occupancy_bb | black_occupancy_bb;
    }

    pub(crate) fn get_side_attacks_bb(&self, attacker_side: Side) -> u64 {
        let mut attacks_bb = chess_consts::EMPTY_BB;

        for piece in Piece::all() {
            let bb = self.get_bb(attacker_side, piece);

            for sq in helpers::get_bits_iter(bb) {
                let square = unsafe { Square::from_u8_unchecked(sq as u8) };

                let piece_attacks_bb = match piece {
                    Piece::Pawn => get_pawn_attacks_mask(attacker_side, square),
                    Piece::Knight => get_knight_attacks_mask(square),
                    Piece::Bishop => get_bishop_attacks_mask(square, self.global_occupancy),
                    Piece::Rook => get_rook_attacks_mask(square, self.global_occupancy),
                    Piece::Queen => get_queen_attacks_mask(square, self.global_occupancy),
                    Piece::King => get_king_attacks_mask(square),
                };
                attacks_bb |= piece_attacks_bb;
            }
        }

        attacks_bb
    }

    pub(crate) fn is_square_attacked(&self, square: Square, attacker_side: Side) -> bool {
        // Checking pawns
        let candidates_pawns_bb = get_pawn_attacks_mask(attacker_side.opposite(), square);
        if candidates_pawns_bb & self.get_bb(attacker_side, Piece::Pawn) != 0 {
            return true;
        }

        // Checking knights
        let candidates_knights_bb = get_knight_attacks_mask(square);
        if candidates_knights_bb & self.get_bb(attacker_side, Piece::Knight) != 0 {
            return true;
        }

        // Checking king
        let candidates_kings_bb = get_king_attacks_mask(square);
        if candidates_kings_bb & self.get_bb(attacker_side, Piece::King) != 0 {
            return true;
        }

        // Checking bishops
        let candidates_bishops_bb = get_bishop_attacks_mask(square, self.global_occupancy);
        if candidates_bishops_bb & self.get_bb(attacker_side, Piece::Bishop) != 0 {
            return true;
        }

        let candidates_rooks_bb = get_rook_attacks_mask(square, self.global_occupancy);
        if candidates_rooks_bb & self.get_bb(attacker_side, Piece::Rook) != 0 {
            return true;
        }

        let candidates_queens_bb = candidates_bishops_bb | candidates_rooks_bb;
        if candidates_queens_bb & self.get_bb(attacker_side, Piece::Queen) != 0 {
            return true;
        }

        false
    }

    pub(crate) fn get_empty_bb(&self) -> u64 {
        !self.global_occupancy
    }

    pub(crate) fn get_occupancy_piece(&self, side: Side, square: Square) -> Option<Piece> {
        let square_mask = square.bit();

        for piece in Piece::all() {
            let piece_bb = self.get_bb(side, piece);

            if piece_bb & square_mask != 0 {
                return Some(piece);
            }
        }

        return None;
    }

    pub(crate) fn get_start_position() -> Board {
        fen_parser::parse_fen_string(chess_consts::fen_strings::START_POS_FEN).unwrap()
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut cells = ['.'; chess_consts::SQUARES_COUNT];

        let mut set = |bb: u64, ch: char| {
            for bit in helpers::get_bits_iter(bb) {
                cells[bit] = ch;
            }
        };

        for side in Side::all() {
            for piece in Piece::all() {
                let bb = self.get_bb(side, piece);
                set(bb, helpers::get_ascii_piece_char(side, piece));
            }
        }

        for rank in (0..chess_consts::BOARD_SIZE).rev() {
            write!(f, "{} ", rank + 1)?;
            for file in 0..chess_consts::BOARD_SIZE {
                let idx = rank * chess_consts::BOARD_SIZE + file;
                write!(f, "{} ", cells[idx])?;
            }
            writeln!(f)?;
        }
        writeln!(f, "  a b c d e f g h")?;
        writeln!(f)?;

        write!(f, "Side: ")?;
        match self.game_state.side_to_move {
            Side::White => writeln!(f, "w")?,
            Side::Black => writeln!(f, "b")?,
        }

        write!(f, "En-passant: ")?;
        match self.game_state.en_passant_square {
            Some(sq) => writeln!(f, "{}", sq)?,
            None => writeln!(f, "-")?,
        }

        writeln!(f, "Castling: {}", self.game_state.castling_state)?;
        writeln!(f, "Half-moves count: {}", self.game_state.half_move_clock)?;
        writeln!(f, "Full moves count: {}", self.game_state.full_moves_count)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CastlingState(pub(crate) u8);

impl Display for CastlingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            if self.0 & Castling::WhiteKingSide.index() != 0 {
                'K'
            } else {
                '-'
            },
            if self.0 & Castling::WhiteQueenSide.index() != 0 {
                'Q'
            } else {
                '-'
            },
            if self.0 & Castling::BlackKingSide.index() != 0 {
                'k'
            } else {
                '-'
            },
            if self.0 & Castling::BlackQueenSide.index() != 0 {
                'q'
            } else {
                '-'
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_board_displaying() {
        let mut board = Board::default();

        *board.get_bb_mut(Side::White, Piece::Pawn) =
            helpers::squares_mask([Square::A2, Square::B2, Square::C2, Square::D2]);
        *board.get_bb_mut(Side::White, Piece::Knight) =
            helpers::squares_mask([Square::B1, Square::G1]);
        *board.get_bb_mut(Side::Black, Piece::Rook) =
            helpers::squares_mask([Square::A8, Square::H8]);

        board.game_state.en_passant_square = Some(Square::E3);

        board.game_state.castling_state =
            CastlingState(Castling::WhiteKingSide.index() | Castling::BlackKingSide.index());

        println!("{board}");
    }

    #[test]
    #[ignore]
    fn test_get_side_attacks_bb() {
        let board = Board::get_start_position();

        println!("White side attacks bb");
        helpers::print_bitboard(board.get_side_attacks_bb(Side::White));

        println!("Black side attacks bb");
        helpers::print_bitboard(board.get_side_attacks_bb(Side::Black));
    }

    #[test]
    fn test_is_square_attacked() {
        // ─────────────────────────────────────────────
        // Start position – pawn attacks
        let board = Board::get_start_position();

        // White pawn attacks
        assert!(board.is_square_attacked(Square::E3, Side::White)); // d2/f2 pawns
        assert!(board.is_square_attacked(Square::D3, Side::White)); // c2/e2 pawns
        assert!(board.is_square_attacked(Square::F3, Side::White)); // e2/g2 pawns
        assert!(!board.is_square_attacked(Square::E4, Side::White)); // pawns don't attack forward

        // Black pawn attacks
        assert!(board.is_square_attacked(Square::E6, Side::Black)); // d7/f7 pawns
        assert!(board.is_square_attacked(Square::D6, Side::Black)); // c7/e7 pawns
        assert!(board.is_square_attacked(Square::F6, Side::Black)); // e7/g7 pawns
        assert!(!board.is_square_attacked(Square::E5, Side::Black));

        // ─────────────────────────────────────────────
        // Knight attacks (single knight on g2)
        let board = fen_parser::parse_fen_string("8/8/8/8/8/8/6N1/8 w - - 0 1").unwrap();
        assert!(board.is_square_attacked(Square::E1, Side::White));
        assert!(board.is_square_attacked(Square::E3, Side::White));
        assert!(board.is_square_attacked(Square::F4, Side::White));
        assert!(board.is_square_attacked(Square::H4, Side::White));
        assert!(!board.is_square_attacked(Square::G3, Side::White));

        // ─────────────────────────────────────────────
        // Bishop blocked by piece
        // Bishop d1, pawn e2 blocks diagonal to f3
        let board = fen_parser::parse_fen_string("8/8/8/8/8/8/4P3/3B4 w - - 0 1").unwrap();
        assert!(board.is_square_attacked(Square::C2, Side::White));
        assert!(!board.is_square_attacked(Square::G4, Side::White));

        // ─────────────────────────────────────────────
        // Rook blocked by own piece
        // Rook a1, pawn a2 blocks file
        let board = fen_parser::parse_fen_string("8/8/8/8/8/8/P7/R7 w - - 0 1").unwrap();
        assert!(board.is_square_attacked(Square::A2, Side::White));
        assert!(!board.is_square_attacked(Square::A3, Side::White));

        // ─────────────────────────────────────────────
        // Queen attacks (center)
        let board = fen_parser::parse_fen_string("8/8/8/8/4Q3/8/8/8 w - - 0 1").unwrap();
        assert!(board.is_square_attacked(Square::E8, Side::White));
        assert!(board.is_square_attacked(Square::A4, Side::White));
        assert!(board.is_square_attacked(Square::H1, Side::White));
        assert!(!board.is_square_attacked(Square::F2, Side::White));

        // ─────────────────────────────────────────────
        // King attacks
        let board = fen_parser::parse_fen_string("8/8/8/8/4K3/8/8/8 w - - 0 1").unwrap();
        assert!(board.is_square_attacked(Square::E5, Side::White));
        assert!(board.is_square_attacked(Square::D4, Side::White));
        assert!(!board.is_square_attacked(Square::E6, Side::White));

        // ─────────────────────────────────────────────
        // Mixed attackers: queen d1 + knight f3
        let board = fen_parser::parse_fen_string("8/8/8/8/8/5N2/8/3Q4 w - - 0 1").unwrap();
        assert!(board.is_square_attacked(Square::H4, Side::White)); // knight from f3
        assert!(board.is_square_attacked(Square::D7, Side::White)); // queen up the file
        assert!(board.is_square_attacked(Square::A1, Side::White));
        assert!(!board.is_square_attacked(Square::A2, Side::White));

        // ─────────────────────────────────────────────
        // Black pieces: symmetry + edge cases

        // Black knight attacks (single knight on g7)
        let board = fen_parser::parse_fen_string("8/6n1/8/8/8/8/8/8 b - - 0 1").unwrap();
        assert!(board.is_square_attacked(Square::E6, Side::Black));
        assert!(board.is_square_attacked(Square::F5, Side::Black));
        assert!(board.is_square_attacked(Square::H5, Side::Black));
        assert!(board.is_square_attacked(Square::E8, Side::Black));
        assert!(!board.is_square_attacked(Square::G6, Side::Black));

        // Black bishop blocked by piece (bishop d8, pawn e7 blocks diagonal)
        let board = fen_parser::parse_fen_string("3b4/4p3/8/8/8/8/8/8 b - - 0 1").unwrap();
        assert!(board.is_square_attacked(Square::C7, Side::Black)); // bishop attacks c7
        assert!(!board.is_square_attacked(Square::G5, Side::Black)); // would be on diagonal, but blocked by e7

        // Black rook blocked by own piece (rook a8, pawn a7 blocks file)
        let board = fen_parser::parse_fen_string("r7/p7/8/8/8/8/8/8 b - - 0 1").unwrap();
        assert!(board.is_square_attacked(Square::A7, Side::Black));
        assert!(!board.is_square_attacked(Square::A6, Side::Black));

        // Black queen attacks (center-ish)
        let board = fen_parser::parse_fen_string("8/8/8/8/8/4q3/8/8 b - - 0 1").unwrap();
        assert!(board.is_square_attacked(Square::E1, Side::Black)); // file down
        assert!(board.is_square_attacked(Square::A3, Side::Black)); // rank
        assert!(board.is_square_attacked(Square::H6, Side::Black)); // diagonal
        assert!(!board.is_square_attacked(Square::F1, Side::Black)); // not attacked square

        // Black king attacks
        let board = fen_parser::parse_fen_string("8/8/8/8/8/4k3/8/8 b - - 0 1").unwrap();
        assert!(board.is_square_attacked(Square::E2, Side::Black));
        assert!(board.is_square_attacked(Square::D3, Side::Black));
        assert!(!board.is_square_attacked(Square::F5, Side::Black));

        // ─────────────────────────────────────────────
        // Pawn edge files: A-file / H-file (only one capture direction)

        // White pawn on a2 attacks only b3
        let board = fen_parser::parse_fen_string("8/8/8/8/8/8/P7/8 w - - 0 1").unwrap();
        assert!(board.is_square_attacked(Square::B3, Side::White));
        assert!(!board.is_square_attacked(Square::A3, Side::White));
        assert!(!board.is_square_attacked(Square::C3, Side::White));

        // Black pawn on h7 attacks only g6
        let board = fen_parser::parse_fen_string("8/7p/8/8/8/8/8/8 b - - 0 1").unwrap();
        assert!(board.is_square_attacked(Square::G6, Side::Black));
        assert!(!board.is_square_attacked(Square::H6, Side::Black));
        assert!(!board.is_square_attacked(Square::F6, Side::Black));

        // ─────────────────────────────────────────────
        // Slider “stop at blocker” semantics (attacked up to blocker, not beyond)

        // White rook a1, black pawn a4 blocks: a4 attacked, a5 not attacked
        let board = fen_parser::parse_fen_string("8/8/8/8/p7/8/8/R7 w - - 0 1").unwrap();
        assert!(board.is_square_attacked(Square::A4, Side::White));
        assert!(!board.is_square_attacked(Square::A5, Side::White));

        // Black bishop h8, white pawn f6 blocks diagonal: f6 attacked, e5 not attacked
        let board = fen_parser::parse_fen_string("7b/8/5P2/8/8/8/8/8 b - - 0 1").unwrap();
        assert!(board.is_square_attacked(Square::F6, Side::Black));
        assert!(!board.is_square_attacked(Square::E5, Side::Black));
    }
}
