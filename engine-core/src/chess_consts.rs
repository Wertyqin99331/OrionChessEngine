use crate::{
    enums::{File, Rank},
    helpers,
};

pub(crate) const SIDES_COUNT: usize = 2;
pub(crate) const BOARD_SIZE: usize = 8;
pub(crate) const SQUARES_COUNT: usize = BOARD_SIZE * BOARD_SIZE;
pub(crate) const PIECE_TYPES_COUNT: usize = 6;
pub(crate) const MOVES_BUF_SIZE: usize = 256;

pub(crate) const MAX_HALF_MOVES_COUNT: u8 = 100;

pub(crate) const EMPTY_BB: u64 = 0u64;

pub(crate) mod fen_strings {
    pub(crate) const EMPTY_BOARD_FEN: &str = "8/8/8/8/8/8/8/8 w - -";
    pub(crate) const START_POS_FEN: &str =
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    pub(crate) const TRICKY_POS_FEN: &str =
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    pub(crate) const KILLER_POS_FEN: &str =
        "rnbqkb1r/pp1p1pPp/8/2p1pP2/1P1P4/3P3P/P1P1P3/RNBQKBNR w KQkq e6 0 1";
    pub(crate) const CMK_POS_FEN: &str =
        "r2q1rk1/ppp2ppp/2n1bn2/2b1p3/3pP3/3P1NPP/PPP1NPB1/R1BQ1RK1 b - - 0 9";
}

/// Bitboard with all bits set except bits on the a file
/// 01111111
/// 01111111
/// 01111111
/// 01111111
/// 01111111
/// 01111111
/// 01111111 01111111
pub(crate) const NOT_A_FILE_BB: u64 = !helpers::file_mask(File::A);

/// Bitboard with all bits set except bits on the h file
pub(crate) const NOT_H_FILE_BB: u64 = !helpers::file_mask(File::H);

/// Bitboard with all bits set except bits on the first rank
pub(crate) const NOT_FIRST_RANK_BB: u64 = !helpers::rank_mask(Rank::R1);

/// Bitboard with all bits set except bits on the eigth rank
pub(crate) const NOT_EIGHTH_RANK_BB: u64 = !helpers::rank_mask(Rank::R8);

pub(crate) const NOT_A_B_FILE_BB: u64 =
    !(helpers::file_mask(File::A) | helpers::file_mask(File::B));

pub(crate) const NOT_G_H_FILE_BB: u64 =
    !(helpers::file_mask(File::G) | helpers::file_mask(File::H));

pub(crate) const NOT_FIRST_SECOND_RANK_BB: u64 =
    !(helpers::rank_mask(Rank::R1) | helpers::rank_mask(Rank::R2));

pub(crate) const NOT_SEVENTH_EIGHTH_RANK_BB: u64 =
    !(helpers::rank_mask(Rank::R7) | helpers::rank_mask(Rank::R8));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_consts() {
        println!("Not a-file bb");
        helpers::print_bitboard(NOT_A_FILE_BB);

        println!("Not h-file bb");
        helpers::print_bitboard(NOT_H_FILE_BB);

        println!("Not first-rank bb");
        helpers::print_bitboard(NOT_FIRST_RANK_BB);

        println!("Not eighth_rank bb");
        helpers::print_bitboard(NOT_EIGHTH_RANK_BB);

        println!("Not a-file b-file bb");
        helpers::print_bitboard(NOT_A_B_FILE_BB);

        println!("Not g-file h-file bb");
        helpers::print_bitboard(NOT_G_H_FILE_BB);

        println!("Not first second rank bb");
        helpers::print_bitboard(NOT_FIRST_SECOND_RANK_BB);

        println!("Not seventh eighth rank bb");
        helpers::print_bitboard(NOT_SEVENTH_EIGHTH_RANK_BB);
    }
}
