use crate::{chess_consts, enums::Square, helpers};

const KNIGHT_ATTACKS_TABLE: [u64; chess_consts::SQUARES_COUNT] = {
    let mut table = [chess_consts::EMPTY_BB; chess_consts::SQUARES_COUNT];

    let mut sq = 0;

    while sq < chess_consts::SQUARES_COUNT as u8 {
        let square = unsafe { Square::from_u8_unchecked(sq) };
        table[sq as usize] = generate_knight_attacks_mask(square);

        sq += 1;
    }

    table
};

/// Get a knight attack table bb based on its square (pre-generated)
pub const fn get_knight_attacks_mask(square: Square) -> u64 {
    KNIGHT_ATTACKS_TABLE[square.index() as usize]
}

/// Generate a knight attack bb
const fn generate_knight_attacks_mask(square: Square) -> u64 {
    let bb = helpers::set_bit(chess_consts::EMPTY_BB, square);

    let mut attack_bb = chess_consts::EMPTY_BB;

    // Up-right jump
    attack_bb |=
        (bb & chess_consts::NOT_SEVENTH_EIGHTH_RANK_BB & chess_consts::NOT_H_FILE_BB) << 17;

    // Right-up jump
    attack_bb |= (bb & chess_consts::NOT_EIGHTH_RANK_BB & chess_consts::NOT_G_H_FILE_BB) << 10;

    // Right-down jump
    attack_bb |= (bb & chess_consts::NOT_FIRST_RANK_BB & chess_consts::NOT_G_H_FILE_BB) >> 6;

    // Down-right jump
    attack_bb |= (bb & chess_consts::NOT_FIRST_SECOND_RANK_BB & chess_consts::NOT_H_FILE_BB) >> 15;

    // Down-left jump
    attack_bb |= (bb & chess_consts::NOT_FIRST_SECOND_RANK_BB & chess_consts::NOT_A_FILE_BB) >> 17;

    // Left-down jump
    attack_bb |= (bb & chess_consts::NOT_FIRST_RANK_BB & chess_consts::NOT_A_B_FILE_BB) >> 10;

    // Left-up jump
    attack_bb |= (bb & chess_consts::NOT_EIGHTH_RANK_BB & chess_consts::NOT_A_B_FILE_BB) << 6;

    // Up-left jump
    attack_bb |=
        (bb & chess_consts::NOT_SEVENTH_EIGHTH_RANK_BB & chess_consts::NOT_A_FILE_BB) << 15;

    attack_bb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knight_attacks_table() {
        for sq in Square::all() {
            println!("{sq}");
            helpers::print_bitboard(get_knight_attacks_mask(sq));
        }
    }
}
