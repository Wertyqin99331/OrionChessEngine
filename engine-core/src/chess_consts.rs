use crate::{
    enums::{File, Rank},
    helpers,
};

pub(crate) const SIDES_COUNT: usize = 2;
pub(crate) const BOARD_SIZE: usize = 8;
pub(crate) const SQUARES_COUNT: usize = BOARD_SIZE * BOARD_SIZE;

pub(crate) const EMPTY_BB: u64 = 0u64;

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
