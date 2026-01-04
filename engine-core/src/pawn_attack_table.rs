use crate::{
    chess_consts,
    enums::{Side, Square},
    helpers,
};

const PAWN_ATTACKS_TABLE: [[u64; chess_consts::SQUARES_COUNT]; chess_consts::SIDES_COUNT] = {
    let mut table =
        [[chess_consts::EMPTY_BB; chess_consts::SQUARES_COUNT]; chess_consts::SIDES_COUNT];

    let mut sq = 0;

    while sq < chess_consts::SQUARES_COUNT as u8 {
        let square = unsafe { Square::from_u8_unchecked(sq) };
        table[Side::White.index() as usize][sq as usize] =
            generate_pawn_attacks_mask(square, Side::White);
        table[Side::Black.index() as usize][sq as usize] =
            generate_pawn_attacks_mask(square, Side::Black);

        sq += 1;
    }

    table
};

/// Get an attack bb based on its position and square (pre-generated)
pub(crate) const fn get_pawn_attacks_mask(side: Side, square: Square) -> u64 {
    PAWN_ATTACKS_TABLE[side.index() as usize][square.index() as usize]
}

/// Get a pawn attack bb based on its position and side
const fn generate_pawn_attacks_mask(square: Square, side: Side) -> u64 {
    let bb = helpers::set_bit(chess_consts::EMPTY_BB, square);

    match side {
        Side::White => {
            (bb & chess_consts::NOT_H_FILE_BB) << 9 | (bb & chess_consts::NOT_A_FILE_BB) << 7
        }
        Side::Black => {
            (bb & chess_consts::NOT_H_FILE_BB) >> 7 | (bb & chess_consts::NOT_A_FILE_BB) >> 9
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_pawn_attacks_table() {
        println!("a1 white pawn attacks");
        helpers::print_bitboard(get_pawn_attacks_mask(Side::White, Square::A1));

        println!("h1 white pawn attacks");
        helpers::print_bitboard(get_pawn_attacks_mask(Side::White, Square::H1));

        println!("e4 white pawn attacks");
        helpers::print_bitboard(get_pawn_attacks_mask(Side::White, Square::E4));

        println!("a8 black pawn attacks");
        helpers::print_bitboard(get_pawn_attacks_mask(Side::Black, Square::A8));

        println!("h8 black pawn attacks");
        helpers::print_bitboard(get_pawn_attacks_mask(Side::Black, Square::H8));

        println!("{} black pawn attacks", Square::E7);
        helpers::print_bitboard(get_pawn_attacks_mask(Side::Black, Square::E7));
    }
}
