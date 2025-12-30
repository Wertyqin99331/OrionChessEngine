use crate::enums::Square;

/// Prints the bitboard to stdout
#[cfg(debug_assertions)]
pub fn print_bitboard(bitboard: u64) {
    for rank in (0..8).rev() {
        for file in 0..8 {
            if file == 0 {
                print!(" {}  ", rank + 1);
            }

            let square = rank * 8 + file;
            let piece = if get_bit(bitboard, Square::try_from(square).unwrap()) != 0 {
                '1'
            } else {
                '0'
            };
            print!(" {piece} ");
        }

        println!();
    }

    println!("\n     a  b  c  d  e  f  g  h");
    println!("\n     Integer value: {bitboard}, hex value: {bitboard:#018x}")
}

/// Returns a bit from the bitboard by the square numbers
/// # Arguments
/// bitboard - the bitboard to get a bit from
/// square [0; 64) - the square to get a bit by
/// # Examples
/// 00001000 3 -> 8, the fourth bit is set, so return it
/// 00001000 4 -> 0, the fifth bit is unset, so 0
pub fn get_bit(bb: u64, square: Square) -> u64 {
    bb & (1u64 << square.index())
}

/// Set a certain bit to 1 on the bitboard
/// # Examples
/// 00000000 0 -> 00000001
/// 10000000 1 -> 10000010
pub fn set_bit(bb: u64, square: Square) -> u64 {
    bb | (1u64 << square.index())
}

/// Pops a certain bit on the bitboard
/// # Examples
/// 00000001 0 -> 00000000
/// If the bit is not set, does nothing
/// 00010000 0 -> 00010000
pub fn pop_bit(bb: u64, square: Square) -> u64 {
    let sq = square.index();
    bb & !(1u64 << sq)
}

/// Flips a certain bit on the bitboard
/// # Examples
/// 00001000 3 -> 00000000
/// 00000000 1 -> 00000010
pub fn flip_bit(bb: u64, square: Square) -> u64 {
    bb ^ (1u64 << square.index())
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
        assert!(get_bit(a1_bitboard, Square::A1) != 0);
        assert!(get_bit(a1_bitboard, Square::A2) == 0);

        let a2_bitboard = Square::A2.bit();
        assert!(get_bit(a2_bitboard, Square::A1) == 0);
        assert!(get_bit(a2_bitboard, Square::A2) != 0);
    }

    #[test]
    fn set_bit_tests() {
        let zero_bb = 0;

        assert!(set_bit(zero_bb, Square::A1) == 1);
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
