use crate::{
    chess_consts::{self, BOARD_SIZE},
    enums::{File, Piece, Rank, Side, Square},
};

/// Prints the bitboard to stdout
#[cfg(any(test, debug_assertions))]
pub fn print_bitboard(bitboard: u64) {
    for rank in (0..8).rev() {
        for file in 0..8 {
            if file == 0 {
                print!(" {}  ", rank + 1);
            }

            let square = rank * 8 + file;
            let piece = if is_bit_set(bitboard, Square::try_from(square).unwrap()) {
                '1'
            } else {
                '0'
            };
            print!("{piece} ");
        }

        println!();
    }

    println!("    a b c d e f g h");
    println!("\n    Integer value: {bitboard}, hex value: {bitboard:#018x}")
}

#[cfg(not(any(test, debug_assertions)))]
pub(crate) fn print_bitboard(_: u64) {}

/// Shows whether a bit at some certain place is set in the bitboard
/// # Arguments
/// bitboard - the bitboard to get a bit from
/// square [0; 64) - the square to check
/// # Examples
/// 00001000 3 -> true, the fourth bit is set, so return true
/// 00001000 4 -> false, the fifth bit is unset, so return false
pub const fn is_bit_set(bb: u64, square: Square) -> bool {
    bb & (1u64 << square.index()) != 0
}

/// Set a certain bit to 1 on the bitboard
/// # Examples
/// 00000000 0 -> 00000001
/// 10000000 1 -> 10000010
pub const fn set_bit(bb: u64, square: Square) -> u64 {
    bb | (1u64 << square.index())
}

/// Pops a certain bit on the bitboard
/// # Examples
/// 00000001 0 -> 00000000
/// If the bit is not set, does nothing
/// 00010000 0 -> 00010000
pub const fn pop_bit(bb: u64, square: Square) -> u64 {
    let sq = square.index();
    bb & !(1u64 << sq)
}

/// Flips a certain bit on the bitboard
/// # Examples
/// 00001000 3 -> 00000000
/// 00000000 1 -> 00000010
pub const fn flip_bit(bb: u64, square: Square) -> u64 {
    bb ^ (1u64 << square.index())
}

/// Returns a bitboard with only certain rank bits set
/// # Examples
/// 7 11111111
///   00000000
///   ........
pub const fn rank_mask(rank: Rank) -> u64 {
    let mut bb = 0u64;
    let mut f = 0;

    while f < chess_consts::BOARD_SIZE as u8 {
        bb |= 1u64 << (rank.index() * BOARD_SIZE as u8 + f);
        f += 1;
    }

    bb
}

/// Returns a bitboard with only certain file bits set
/// # Examples
/// 0 10000000
///   10000000
///   .......
pub const fn file_mask(file: File) -> u64 {
    let mut bb = 0u64;
    let mut r = 0;
    while r < chess_consts::BOARD_SIZE as u8 {
        bb |= 1u64 << (r * chess_consts::BOARD_SIZE as u8 + file.index());
        r += 1;
    }
    bb
}

/// Returns a mask with only this (rank, file) bit set
pub const fn square_mask(rank: u8, file: u8) -> u64 {
    1u64 << (rank * chess_consts::BOARD_SIZE as u8 + file)
}

pub fn squares_mask(squares: impl IntoIterator<Item = Square>) -> u64 {
    let mut bb = 0;

    for sq in squares.into_iter() {
        bb |= sq.bit();
    }

    bb
}

/// Returns an iterator over indexes of set bits from LSB
/// # Examples
/// 1010 -> 1 3
#[inline]
pub(crate) fn get_bits_iter(bb: u64) -> impl Iterator<Item = usize> {
    let mut x = bb;

    std::iter::from_fn(move || {
        if x == 0 {
            None
        } else {
            let sq = x.trailing_zeros() as usize;
            x &= x - 1;
            Some(sq)
        }
    })
}

/// Returns an iterator over indexes of set bits from LSB
/// # Examples
/// 1010 -> B1 D1
#[inline]
pub(crate) fn get_squares_iter(bb: u64) -> impl Iterator<Item = Square> {
    let mut x = bb;

    std::iter::from_fn(move || {
        if x == 0 {
            None
        } else {
            let sq = unsafe { Square::from_u8_unchecked(x.trailing_zeros() as u8) };
            x &= x - 1;
            Some(sq)
        }
    })
}

#[inline]
pub(crate) fn get_ascii_piece_char(side: Side, piece: Piece) -> char {
    const ASCII_PIECE_CHARS: [char; chess_consts::PIECE_TYPES_COUNT * 2] =
        ['P', 'N', 'B', 'R', 'Q', 'K', 'p', 'n', 'b', 'r', 'q', 'k'];

    ASCII_PIECE_CHARS
        [(side.index() * chess_consts::PIECE_TYPES_COUNT as u8 + piece.index()) as usize]
}

#[cfg(test)]
mod tests {
    use crate::enums::Square;

    use super::*;

    #[test]
    #[ignore]
    fn print_bitboard_test() {
        let a1_bitboard = Square::A1.bit();
        let a2_bitboard = Square::A2.bit();
        let some_bitboard = Square::A1.bit() | Square::H1.bit() | Square::E4.bit();

        for bb in [a1_bitboard, a2_bitboard, some_bitboard] {
            print_bitboard(bb);
            println!();
        }
    }

    #[test]
    fn get_bit_tests() {
        let a1_bitboard = Square::A1.bit();
        assert_eq!(is_bit_set(a1_bitboard, Square::A1), true);
        assert_eq!(is_bit_set(a1_bitboard, Square::A2), false);

        let a2_bitboard = Square::A2.bit();
        assert_eq!(is_bit_set(a2_bitboard, Square::A1), false);
        assert_eq!(is_bit_set(a2_bitboard, Square::A2), true);
    }

    #[test]
    fn set_bit_tests() {
        let zero_bb = 0;

        assert!(set_bit(zero_bb, Square::A1) == Square::A1.bit());
        assert!(set_bit(zero_bb, Square::E4) == Square::E4.bit());

        assert!(set_bit(Square::E1.bit(), Square::F1) == Square::E1.bit() | Square::F1.bit());
    }

    #[test]
    fn pop_bit_tests() {
        assert!(pop_bit(Square::A1.bit(), Square::A1) == 0);
        assert!(pop_bit(Square::A2.bit(), Square::A1) == Square::A2.bit());
    }

    #[test]
    fn flip_bit_tests() {
        assert!(flip_bit(Square::A1.bit(), Square::A1) == 0);
        assert!(flip_bit(Square::H8.bit(), Square::A1) == Square::A1.bit() | Square::H8.bit());
    }
}
