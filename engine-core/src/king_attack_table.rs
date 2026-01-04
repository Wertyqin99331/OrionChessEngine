use crate::{chess_consts, enums::Square, helpers};

const KING_ATTACKS_TABLE: [u64; chess_consts::SQUARES_COUNT] = {
    let mut table = [chess_consts::EMPTY_BB; chess_consts::SQUARES_COUNT];

    let mut sq = 0;

    while sq < chess_consts::SQUARES_COUNT as u8 {
        let square = unsafe { Square::from_u8_unchecked(sq) };
        table[sq as usize] = generate_king_attacks_mask(square);

        sq += 1;
    }

    table
};

pub(crate) const fn get_king_attacks_mask(square: Square) -> u64 {
    KING_ATTACKS_TABLE[square.index() as usize]
}

const fn generate_king_attacks_mask(square: Square) -> u64 {
    let bb = helpers::set_bit(chess_consts::EMPTY_BB, square);
    let mut attack_bb = chess_consts::EMPTY_BB;

    // Up-right
    attack_bb |= (bb & chess_consts::NOT_EIGHTH_RANK_BB & chess_consts::NOT_H_FILE_BB) << 9;

    // Right
    attack_bb |= (bb & chess_consts::NOT_H_FILE_BB) << 1;

    // Down-right
    attack_bb |= (bb & chess_consts::NOT_FIRST_RANK_BB & chess_consts::NOT_H_FILE_BB) >> 7;

    // Down
    attack_bb |= (bb & chess_consts::NOT_FIRST_RANK_BB) >> 8;

    // Down-left
    attack_bb |= (bb & chess_consts::NOT_FIRST_RANK_BB & chess_consts::NOT_A_FILE_BB) >> 9;

    // Left
    attack_bb |= (bb & chess_consts::NOT_A_FILE_BB) >> 1;

    // Up-left
    attack_bb |= (bb & chess_consts::NOT_EIGHTH_RANK_BB & chess_consts::NOT_A_FILE_BB) << 7;

    // Up
    attack_bb |= (bb & chess_consts::NOT_EIGHTH_RANK_BB) << 8;

    attack_bb
}

mod tests {
    use super::*;

    #[test]
    fn test_king_attacks_table() {
        for sq in Square::all() {
            let sq = Square::try_from(sq).unwrap();

            println!("{sq}");
            helpers::print_bitboard(get_king_attacks_mask(sq));
        }
    }
}
