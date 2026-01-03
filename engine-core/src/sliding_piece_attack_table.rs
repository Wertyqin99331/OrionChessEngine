use std::sync::LazyLock;

use crate::{
    chess_consts,
    enums::{Piece, Square},
    helpers,
    random_generator::XorShift64Star,
};

const BISHOP_RELEVANT_OCCUPANCY_MASKS: [u64; chess_consts::SQUARES_COUNT] = {
    let mut relevant_masks = [0u64; chess_consts::SQUARES_COUNT];
    let mut sq = 0;

    while sq < chess_consts::SQUARES_COUNT {
        let square = unsafe { Square::from_u8_unchecked(sq as u8) };

        let relevant_occupancy_mask = generate_relevant_bishop_occupancy_mask(square);

        relevant_masks[sq] = relevant_occupancy_mask;

        sq += 1;
    }

    relevant_masks
};

const BISHOP_RELEVANT_BIT_COUNTS: [u8; chess_consts::SQUARES_COUNT] = {
    let mut counts = [0; chess_consts::SQUARES_COUNT];
    let mut sq = 0;

    while sq < chess_consts::SQUARES_COUNT {
        counts[sq] = BISHOP_RELEVANT_OCCUPANCY_MASKS[sq].count_ones() as u8;

        sq += 1;
    }

    counts
};

const ROOK_RELEVANT_OCCUPANCY_MASKS: [u64; chess_consts::SQUARES_COUNT] = {
    let mut relevant_masks = [0u64; chess_consts::SQUARES_COUNT];
    let mut sq = 0;

    while sq < chess_consts::SQUARES_COUNT {
        let square = unsafe { Square::from_u8_unchecked(sq as u8) };

        let relevant_occupancy_mask = generate_relevant_rook_occupancy_mask(square);

        relevant_masks[sq] = relevant_occupancy_mask;

        sq += 1;
    }

    relevant_masks
};

const ROOK_RELEVANT_BIT_COUNTS: [u8; chess_consts::SQUARES_COUNT] = {
    let mut counts = [0; chess_consts::SQUARES_COUNT];
    let mut sq = 0;

    while sq < chess_consts::SQUARES_COUNT {
        counts[sq] = ROOK_RELEVANT_OCCUPANCY_MASKS[sq].count_ones() as u8;

        sq += 1;
    }

    counts
};

static BISHOP_MAGIC_NUMBERS: LazyLock<[u64; chess_consts::SQUARES_COUNT]> = LazyLock::new(|| {
    let mut magic_numbers = [0u64; chess_consts::SQUARES_COUNT];

    let mut sq = 0;

    while sq < chess_consts::SQUARES_COUNT {
        let square = unsafe { Square::from_u8_unchecked(sq as u8) };

        let magic_number = find_magic_number(square, Piece::Bishop);

        magic_numbers[sq] = magic_number.unwrap();

        sq += 1;
    }

    magic_numbers
});

static ROOK_MAGIC_NUMBERS: LazyLock<[u64; chess_consts::SQUARES_COUNT]> = LazyLock::new(|| {
    let mut magic_numbers = [0u64; chess_consts::SQUARES_COUNT];

    let mut sq = 0;

    while sq < chess_consts::SQUARES_COUNT {
        let square = unsafe { Square::from_u8_unchecked(sq as u8) };

        let magic_number = find_magic_number(square, Piece::Rook);

        magic_numbers[sq] = magic_number.unwrap();

        sq += 1;
    }

    magic_numbers
});

static BISHOP_ATTACKS_TABLE: LazyLock<[[u64; 512]; chess_consts::SQUARES_COUNT]> =
    LazyLock::new(|| {
        let mut attacks_table = [[0; 512]; chess_consts::SQUARES_COUNT];

        for square in Square::all() {
            let sq_index = square.index() as usize;
            let relevant_bits_count = BISHOP_RELEVANT_BIT_COUNTS[sq_index];
            let relevant_occupancy_mask = BISHOP_RELEVANT_OCCUPANCY_MASKS[sq_index];

            let occupancy_indicies = 2u32.pow(relevant_bits_count as u32);

            for occupancy_index in 0..occupancy_indicies {
                let blocker_mask = build_blocker_mask(occupancy_index, relevant_occupancy_mask);

                let shift = 64u32 - (relevant_bits_count as u32);
                let magic_index =
                    blocker_mask.wrapping_mul(BISHOP_MAGIC_NUMBERS[sq_index]) >> shift;
                attacks_table[sq_index][magic_index as usize] =
                    generate_bishop_attacks_mask(square, blocker_mask);
            }
        }

        attacks_table
    });

static ROOK_ATTACKS_TABLE: LazyLock<Box<[[u64; 4096]; chess_consts::SQUARES_COUNT]>> =
    LazyLock::new(|| {
        let flat: Box<[u64]> = vec![0u64; 4096 * chess_consts::SQUARES_COUNT].into_boxed_slice();
        let ptr = Box::into_raw(flat) as *mut [[u64; 4096]; chess_consts::SQUARES_COUNT];
        let mut attacks_table: Box<[[u64; 4096]; chess_consts::SQUARES_COUNT]> =
            unsafe { Box::from_raw(ptr) };

        for square in Square::all() {
            let sq_index = square.index() as usize;
            let relevant_bits_count = ROOK_RELEVANT_BIT_COUNTS[sq_index];
            let relevant_occupancy_mask = ROOK_RELEVANT_OCCUPANCY_MASKS[sq_index];

            let occupancy_indicies = 2u32.pow(relevant_bits_count as u32);

            for occupancy_index in 0..occupancy_indicies {
                let blocker_mask = build_blocker_mask(occupancy_index, relevant_occupancy_mask);

                let shift = 64u32 - (relevant_bits_count as u32);
                let magic_index = blocker_mask.wrapping_mul(ROOK_MAGIC_NUMBERS[sq_index]) >> shift;

                attacks_table[sq_index][magic_index as usize] =
                    generate_rook_attacks_mask(square, blocker_mask);
            }
        }

        attacks_table
    });

pub(crate) fn get_bishop_attacks_mask(square: Square, mut occupancy: u64) -> u64 {
    let square_index = square.index() as usize;
    occupancy &= BISHOP_RELEVANT_OCCUPANCY_MASKS[square_index];

    let magic_index = (occupancy.wrapping_mul(BISHOP_MAGIC_NUMBERS[square_index]))
        >> (64 - BISHOP_RELEVANT_BIT_COUNTS[square_index]);

    BISHOP_ATTACKS_TABLE[square_index][magic_index as usize]
}

pub(crate) fn get_rook_attacks_mask(square: Square, mut occupancy: u64) -> u64 {
    let square_index = square.index() as usize;
    occupancy &= ROOK_RELEVANT_OCCUPANCY_MASKS[square_index];

    let magic_index = (occupancy.wrapping_mul(ROOK_MAGIC_NUMBERS[square_index]))
        >> (64 - ROOK_RELEVANT_BIT_COUNTS[square_index]);

    ROOK_ATTACKS_TABLE[square_index][magic_index as usize]
}

pub(crate) const fn generate_relevant_bishop_occupancy_mask(square: Square) -> u64 {
    let mut attacks_bb = chess_consts::EMPTY_BB;

    let (target_rank, target_file) = (square.rank(), square.file());

    // Up-right
    let mut rank = target_rank + 1;
    let mut file = target_file + 1;

    while rank < (chess_consts::BOARD_SIZE - 1) as u8 && file < (chess_consts::BOARD_SIZE - 1) as u8
    {
        attacks_bb |= helpers::square_mask(rank, file);
        rank += 1;
        file += 1;
    }

    // Up-left
    rank = target_rank + 1;
    file = if target_file == 0 { 0 } else { target_file - 1 };

    while rank < (chess_consts::BOARD_SIZE - 1) as u8 && file > 0 {
        attacks_bb |= helpers::square_mask(rank, file);
        rank += 1;
        file -= 1;
    }

    // Down-right
    rank = if target_rank == 0 { 0 } else { target_rank - 1 };
    file = target_file + 1;

    while rank > 0 && file < (chess_consts::BOARD_SIZE - 1) as u8 {
        attacks_bb |= helpers::square_mask(rank, file);
        rank -= 1;
        file += 1;
    }

    // Down-left
    rank = if target_rank == 0 { 0 } else { target_rank - 1 };
    file = if target_file == 0 { 0 } else { target_file - 1 };

    while rank > 0 && file > 0 {
        attacks_bb |= helpers::square_mask(rank, file);
        rank -= 1;
        file -= 1;
    }

    attacks_bb
}

pub(crate) const fn generate_relevant_rook_occupancy_mask(square: Square) -> u64 {
    let (target_rank, target_file) = (square.rank(), square.file());

    let mut attacks_bb = chess_consts::EMPTY_BB;

    // Up
    let mut rank = target_rank + 1;
    let mut file = target_file;

    while rank < (chess_consts::BOARD_SIZE - 1) as u8 {
        attacks_bb |= helpers::square_mask(rank, file);
        rank += 1;
    }

    // Right
    rank = target_rank;
    file = target_file + 1;

    while file < (chess_consts::BOARD_SIZE - 1) as u8 {
        attacks_bb |= helpers::square_mask(rank, file);
        file += 1;
    }

    // Down
    rank = if target_rank == 0 { 0 } else { target_rank - 1 };
    file = target_file;

    while rank > 0 {
        attacks_bb |= helpers::square_mask(rank, file);
        rank -= 1;
    }

    // Left
    rank = target_rank;
    file = if target_file == 0 { 0 } else { target_file - 1 };

    while file > 0 {
        attacks_bb |= helpers::square_mask(rank, file);
        file -= 1;
    }

    attacks_bb
}

const fn generate_bishop_attacks_mask(square: Square, blockers: u64) -> u64 {
    let mut attacks_bb = chess_consts::EMPTY_BB;

    let (target_rank, target_file) = (square.rank(), square.file());

    // Up-right
    let mut rank = target_rank as i8 + 1;
    let mut file = target_file as i8 + 1;

    while rank < chess_consts::BOARD_SIZE as i8 && file < chess_consts::BOARD_SIZE as i8 {
        let square_mask = helpers::square_mask(rank as u8, file as u8);
        attacks_bb |= square_mask;

        if (square_mask & blockers) != 0 {
            break;
        }

        rank += 1;
        file += 1;
    }

    // Up-left
    rank = target_rank as i8 + 1;
    file = target_file as i8 - 1;

    while rank < chess_consts::BOARD_SIZE as i8 && file >= 0 {
        let square_mask = helpers::square_mask(rank as u8, file as u8);
        attacks_bb |= square_mask;

        if (square_mask & blockers) != 0 {
            break;
        }

        rank += 1;
        file -= 1;
    }

    // Down-right
    rank = target_rank as i8 - 1;
    file = target_file as i8 + 1;

    while rank >= 0 && file < chess_consts::BOARD_SIZE as i8 {
        let square_mask = helpers::square_mask(rank as u8, file as u8);
        attacks_bb |= square_mask;

        if (square_mask & blockers) != 0 {
            break;
        }

        rank -= 1;
        file += 1;
    }

    // Down-left
    rank = target_rank as i8 - 1;
    file = target_file as i8 - 1;

    while rank >= 0 && file >= 0 {
        let square_mask = helpers::square_mask(rank as u8, file as u8);
        attacks_bb |= square_mask;

        if (square_mask & blockers) != 0 {
            break;
        }

        rank -= 1;
        file -= 1;
    }

    attacks_bb
}

const fn generate_rook_attacks_mask(square: Square, blockers: u64) -> u64 {
    let mut attacks_bb = chess_consts::EMPTY_BB;

    let (target_rank, target_file) = (square.rank(), square.file());

    // Up
    let mut rank = target_rank as i8 + 1;
    let mut file = target_file as i8;

    while rank < chess_consts::BOARD_SIZE as i8 {
        let square_mask = helpers::square_mask(rank as u8, file as u8);
        attacks_bb |= square_mask;

        if (square_mask & blockers) != 0 {
            break;
        }

        rank += 1;
    }

    // Right
    rank = target_rank as i8;
    file = target_file as i8 + 1;

    while file < chess_consts::BOARD_SIZE as i8 {
        let square_mask = helpers::square_mask(rank as u8, file as u8);
        attacks_bb |= square_mask;

        if (square_mask & blockers) != 0 {
            break;
        }

        file += 1;
    }

    // Down
    rank = target_rank as i8 - 1;
    file = target_file as i8;

    while rank >= 0 {
        let square_mask = helpers::square_mask(rank as u8, file as u8);
        attacks_bb |= square_mask;

        if (square_mask & blockers) != 0 {
            break;
        }

        rank -= 1;
    }

    // Left
    rank = target_rank as i8;
    file = target_file as i8 - 1;

    while file >= 0 {
        let square_mask = helpers::square_mask(rank as u8, file as u8);
        attacks_bb |= square_mask;

        if (square_mask & blockers) != 0 {
            break;
        }

        file -= 1;
    }

    attacks_bb
}

pub(crate) const fn build_blocker_mask(index: u32, mut relevant_mask: u64) -> u64 {
    let mut blocker = chess_consts::EMPTY_BB;
    let bits = relevant_mask.count_ones();

    let mut i = 0;
    while i < bits {
        let square = relevant_mask.trailing_zeros();

        if (index & (1u32 << i)) != 0 {
            blocker |= 1u64 << square;
        }

        relevant_mask &= relevant_mask - 1;
        i += 1;
    }

    blocker
}

const fn find_magic_number(square: Square, piece: Piece) -> Option<u64> {
    match piece {
        Piece::Bishop | Piece::Rook => {}
        _ => panic!("find_magic_number function works only with bishop or rook piece types"),
    }

    let mut occupancies = [0u64; 4096];
    let mut attacks = [0u64; 4096];
    let mut used_attacks;

    let relevant_occupancy_mask = match piece {
        Piece::Bishop => generate_relevant_bishop_occupancy_mask(square),
        Piece::Rook => generate_relevant_rook_occupancy_mask(square),
        _ => unreachable!(),
    };

    let relevant_bits_count = relevant_occupancy_mask.count_ones();
    let occupancy_indicies = 2u64.pow(relevant_bits_count);

    let mut index = 0;
    while index < occupancy_indicies as usize {
        occupancies[index] = build_blocker_mask(index as u32, relevant_occupancy_mask);

        attacks[index] = match piece {
            Piece::Bishop => generate_bishop_attacks_mask(square, occupancies[index]),
            Piece::Rook => generate_rook_attacks_mask(square, occupancies[index]),
            _ => unreachable!(),
        };

        index += 1;
    }

    let mut rng_generator = XorShift64Star::new();
    let mut random_index = 0;
    while random_index < 100_000_000 {
        random_index += 1;
        let magic_number = rng_generator.generate_magic_number_candidate();

        // Check that first 8 bits contain at least MIN_HIGH_BITS_SET to remove "mostly-zero" magics
        const HIGH_8_BITS_MASK: u64 = 0xFF00_0000_0000_0000;
        const MIN_HIGH_BITS_SET: u32 = 6;

        let mixed = relevant_occupancy_mask.wrapping_mul(magic_number);
        let high_bits = (mixed & HIGH_8_BITS_MASK).count_ones();

        if high_bits < MIN_HIGH_BITS_SET {
            continue;
        }

        used_attacks = [0u64; 4096];
        let mut index = 0usize;

        let mut fail = false;
        while index < occupancy_indicies as usize {
            let shift = 64 - relevant_bits_count;
            let magic_index = occupancies[index].wrapping_mul(magic_number) >> shift;

            // If no occupancy has landed here, ok
            if used_attacks[magic_index as usize] == 0 {
                used_attacks[magic_index as usize] = attacks[index];
            } else if used_attacks[magic_index as usize] == attacks[index] {
                // If occupancy with the same attack table has landed here, it is ok too
            } else {
                fail = true;
                break;
            }

            index += 1;
        }

        if !fail {
            return Some(magic_number);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use crate::helpers;

    use super::*;

    #[test]
    #[ignore]
    fn test_generate_relevant_bishop_occupancy_mask() {
        for sq in Square::all() {
            println!("{}", sq);
            helpers::print_bitboard(generate_relevant_bishop_occupancy_mask(sq));
        }
    }

    #[test]
    #[ignore]
    fn test_generate_relevant_rook_occupancy_mask() {
        for sq in Square::all() {
            println!("{}", sq);
            helpers::print_bitboard(generate_relevant_rook_occupancy_mask(sq));
        }
    }

    #[test]
    #[ignore]
    fn test_generate_bishop_attacks_mask() {
        for sq in Square::all() {
            println!("{}", sq);
            helpers::print_bitboard(generate_bishop_attacks_mask(sq, chess_consts::EMPTY_BB));
        }

        println!("{} with blocker on {}", Square::E4, Square::G6);
        helpers::print_bitboard(generate_bishop_attacks_mask(Square::E4, Square::G6.bit()));

        println!(
            "{} with blockers on {} and {}",
            Square::D4,
            Square::B6,
            Square::C3
        );
        helpers::print_bitboard(generate_bishop_attacks_mask(
            Square::D4,
            Square::B6.bit() | Square::C3.bit(),
        ));
    }

    #[test]
    #[ignore]
    fn test_generate_rook_attacks_mask() {
        for sq in Square::all() {
            println!("{}", sq);
            helpers::print_bitboard(generate_rook_attacks_mask(sq, chess_consts::EMPTY_BB));
        }

        println!("{} with blocker on {}", Square::E1, Square::G1);
        helpers::print_bitboard(generate_rook_attacks_mask(Square::E1, Square::G1.bit()));

        println!(
            "{} with blockers on {} and {}",
            Square::E4,
            Square::B4,
            Square::E6
        );
        helpers::print_bitboard(generate_rook_attacks_mask(
            Square::E4,
            Square::B4.bit() | Square::E6.bit(),
        ));
    }

    #[test]
    #[ignore]
    fn test_system_supports_pext_operation() {
        if std::is_x86_feature_detected!("bmi2") {
            println!("Pext is available");
        } else {
            println!("Pext is unavailable");
        }
    }

    #[test]
    #[ignore]
    fn test_build_blocker_mask() {
        let rook_relevant_occupancy_mask = generate_relevant_rook_occupancy_mask(Square::A1);

        for i in (0..2i32.pow(rook_relevant_occupancy_mask.count_ones())).rev() {
            println!("Index is {i}");
            helpers::print_bitboard(build_blocker_mask(i as u32, rook_relevant_occupancy_mask));
        }
    }

    #[test]
    #[ignore]
    fn test_bishop_and_rook_relevant_bit_counts_tables() {
        println!("Bishop relevant bit counts table");
        for i in 0..chess_consts::SQUARES_COUNT {
            print!("{} ", BISHOP_RELEVANT_BIT_COUNTS[i]);
            if i % chess_consts::BOARD_SIZE == 7 {
                println!();
            }
        }

        println!();

        println!("Rook relevant bit counts table");
        for i in 0..chess_consts::SQUARES_COUNT {
            print!("{} ", ROOK_RELEVANT_BIT_COUNTS[i]);
            if i % chess_consts::BOARD_SIZE == 7 {
                println!();
            }
        }
    }

    #[test]
    #[ignore]
    fn test_find_magic_number() {
        let start = Instant::now();

        for sq in Square::all() {
            let bishop_magic_number = BISHOP_MAGIC_NUMBERS[sq.index() as usize];
            let rook_magic_number = ROOK_MAGIC_NUMBERS[sq.index() as usize];

            println!(
                "Square: {sq}, Bishop magic number: {:?}, rook magic number: {:?}",
                bishop_magic_number, rook_magic_number
            );
        }

        println!("Elapsed: {:?}", start.elapsed().as_millis());
    }

    #[test]
    #[ignore]
    fn test_bishop_rook_attacks_tables() {
        println!("Bishop a1 with B2 blocker");
        helpers::print_bitboard(get_bishop_attacks_mask(Square::A1, Square::B2.bit()));

        println!("Bishop a1 with C3 blocker");
        helpers::print_bitboard(get_bishop_attacks_mask(Square::A1, Square::C3.bit()));

        println!("Rook a1 with B1 blocker");
        helpers::print_bitboard(get_rook_attacks_mask(Square::A1, Square::B1.bit()));

        println!("Rook a1 with C1  blocker");
        helpers::print_bitboard(get_rook_attacks_mask(Square::A1, Square::C1.bit()));
    }
}
