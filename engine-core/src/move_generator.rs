use std::str::FromStr;

use crate::{
    board::Board,
    chess_consts,
    enums::{Move, MoveFlags, Piece, Rank, Side, Square},
    helpers,
    pawn_attack_table::get_pawn_attacks_mask,
};

pub(crate) fn generate_pseudo_legal_moves(board: &Board, side: Side) -> Vec<Move> {
    let mut moves = vec![];

    moves.append(&mut generate_pseudo_legal_pawn_moves(board, side));

    moves
}

fn generate_pseudo_legal_pawn_moves(board: &Board, side: Side) -> Vec<Move> {
    let mut moves = vec![];

    // Generate pawn moves
    let pawn_bb = board.get_bb(side, Piece::Pawn);
    let square_shift = if side == Side::White { 8 } else { -8 };

    // Generate quiet moves
    let pawn_one_step_bb = push_pawn(pawn_bb, side) & board.get_empty_bb();

    let promotion_mask = helpers::rank_mask(side.get_promotion_rank());
    let pawn_one_step_not_promotion_bb = pawn_one_step_bb & (!promotion_mask);
    let pawn_one_step_promotion_bb = pawn_one_step_bb & promotion_mask;

    // One step moves with no promotion
    for bit in helpers::get_bits_iter(pawn_one_step_not_promotion_bb) {
        let to = unsafe { Square::from_u8_unchecked(bit as u8) };
        let from = unsafe { Square::from_u8_unchecked((bit as i8 - square_shift) as u8) };

        moves.push(Move::Normal {
            from,
            to,
            piece: Piece::Pawn,
            captured: None,
            promo: None,
            flags: MoveFlags::empty(),
        });
    }

    // One step moves with promotion
    for bit in helpers::get_bits_iter(pawn_one_step_promotion_bb) {
        let to = unsafe { Square::from_u8_unchecked(bit as u8) };
        let from = unsafe { Square::from_u8_unchecked((bit as i8 - square_shift) as u8) };

        for promotion_piece in Piece::PROMOTION_PIECES {
            let mv = Move::Normal {
                from,
                to,
                piece: Piece::Pawn,
                captured: None,
                promo: Some(promotion_piece),
                flags: MoveFlags::empty(),
            };
            moves.push(mv);
        }
    }

    // Two steps moves
    let one_step_mask = helpers::rank_mask(if side == Side::White {
        Rank::R3
    } else {
        Rank::R6
    });
    let pawn_two_steps_bb =
        push_pawn(pawn_one_step_bb & one_step_mask, side) & board.get_empty_bb();

    for bit in helpers::get_bits_iter(pawn_two_steps_bb) {
        let to = unsafe { Square::from_u8_unchecked(bit as u8) };
        let from = unsafe { Square::from_u8_unchecked((bit as i8 - 2 * square_shift) as u8) };

        let mv = Move::Normal {
            from,
            to,
            piece: Piece::Pawn,
            captured: None,
            promo: None,
            flags: MoveFlags::DOUBLE_PUSH,
        };
        moves.push(mv);
    }

    // Normal attacks
    for bit in helpers::get_bits_iter(pawn_bb) {
        let from = unsafe { Square::from_u8_unchecked(bit as u8) };

        let attacks_bb = get_pawn_attacks_mask(side, from);
        let valid_attacks_bb = attacks_bb & board.get_occupancy_bb(side.opposite());

        for attacks_bit in helpers::get_bits_iter(valid_attacks_bb) {
            let to = unsafe { Square::from_u8_unchecked(attacks_bit as u8) };
            let capture_piece = board.get_occupancy_piece(side.opposite(), to).unwrap();

            if to.rank() == side.get_promotion_rank() {
                for promotion_piece in Piece::PROMOTION_PIECES {
                    let mv = Move::Normal {
                        from,
                        to,
                        piece: Piece::Pawn,
                        captured: Some(capture_piece),
                        promo: Some(promotion_piece),
                        flags: MoveFlags::empty(),
                    };
                    moves.push(mv);
                }
            } else {
                let mv = Move::Normal {
                    from,
                    to,
                    piece: Piece::Pawn,
                    captured: Some(capture_piece),
                    promo: None,
                    flags: MoveFlags::empty(),
                };
                moves.push(mv);
            }
        }
    }

    moves
}

#[inline(always)]
fn push_pawn(bb: u64, side: Side) -> u64 {
    if side == Side::White {
        bb << chess_consts::BOARD_SIZE
    } else {
        bb >> chess_consts::BOARD_SIZE
    }
}

#[cfg(test)]
mod tests {
    use crate::fen_parser;

    use super::*;

    #[test]
    fn test_generate_pseudo_legal_pawn_quiet_moves() {
        println!("Start position");
        let board = Board::get_start_position();

        println!(
            "White: {:?}",
            generate_pseudo_legal_pawn_moves(&board, Side::White)
        );
        println!(
            "Black: {:?}",
            generate_pseudo_legal_pawn_moves(&board, Side::Black)
        );
        println!();

        // white pawns - b6, g7
        println!("White pawns: [b6, g7]");
        let board = fen_parser::parse_fen_string("8/6P1/1P6/8/8/8/8/8 w - - 0 1").unwrap();

        println!(
            "White: {:?}",
            generate_pseudo_legal_pawn_moves(&board, Side::White)
        );
    }

    #[test]
    fn test_generate_pseudo_legal_pawn_capture_moves() {
        println!("White pawns: [d3, g3], black pawns: [c4, f4, h4]");
        let board = fen_parser::parse_fen_string("8/8/8/8/2p2p1p/3P2P1/8/8 w - - 0 1").unwrap();

        println!(
            "White: {:?}",
            generate_pseudo_legal_pawn_moves(&board, Side::White)
        );
        println!(
            "Black: {:?}",
            generate_pseudo_legal_pawn_moves(&board, Side::Black)
        );
        println!();

        let board =
            fen_parser::parse_fen_string("4q3/3P4/7p/6P1/1p6/P7/5p2/4Q3 w - - 0 1").unwrap();

        println!(
            "White: {:?}",
            generate_pseudo_legal_pawn_moves(&board, Side::White)
        );
        println!(
            "Black: {:?}",
            generate_pseudo_legal_pawn_moves(&board, Side::Black)
        );
        println!();
    }
}
