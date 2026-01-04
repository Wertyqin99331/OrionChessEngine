use crate::{
    board::Board,
    chess_consts,
    enums::{CastlingSide, Move, MoveFlags, Piece, Rank, Side, Square},
    helpers,
    king_attack_table::get_king_attacks_mask,
    knight_attack_table::get_knight_attacks_mask,
    pawn_attack_table::get_pawn_attacks_mask,
    sliding_piece_attack_table::{
        get_bishop_attacks_mask, get_queen_attacks_mask, get_rook_attacks_mask,
    },
};

impl Board {
    pub(crate) fn generate_pseudo_legal_moves(&self, side: Side) -> Vec<Move> {
        let mut moves = vec![];

        let generate_pseudo_legal_moves_handlers = [
            generate_pseudo_legal_pawn_moves,
            generate_pseudo_legal_knight_moves,
            generate_pseudo_legal_bishop_moves,
            generate_pseudo_legal_rook_moves,
            generate_pseudo_legal_queen_moves,
            generate_pseudo_legal_king_moves,
            generate_castling_moves,
        ];

        for handler in generate_pseudo_legal_moves_handlers {
            moves.append(&mut handler(self, side));
        }

        moves
    }

    pub(crate) fn generate_legal_moves(&mut self, side: Side) -> Vec<Move> {
        let pseudo_legal_moves = self.generate_pseudo_legal_moves(side);
        let mut legal_moves = vec![];

        for mv in pseudo_legal_moves {
            self.make_move(mv);

            if !self.is_in_check(side) {
                legal_moves.push(mv);
            }

            self.unmake_move();
        }

        legal_moves
    }
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

    // Check whether the current en-passant square is from the opposite side
    let en_passant_sq_bb = if let Some(en_passant_sq) = board.game_state.en_passant_square
        && Square::is_en_passant_target_for(en_passant_sq, side)
    {
        en_passant_sq.bit()
    } else {
        chess_consts::EMPTY_BB
    };

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

        // En-passant
        if en_passant_sq_bb != 0 {
            let attack_en_passant_bb = attacks_bb & en_passant_sq_bb;
            if attack_en_passant_bb != 0 {
                let bit = attack_en_passant_bb.trailing_zeros();
                let to = unsafe { Square::from_u8_unchecked(bit as u8) };

                let mv = Move::Normal {
                    from,
                    to,
                    piece: Piece::Pawn,
                    captured: Some(Piece::Pawn),
                    promo: None,
                    flags: MoveFlags::EN_PASSANT,
                };
                moves.push(mv);
            }
        }
    }

    moves
}

fn generate_leaper_pseudo_legal_moves(
    board: &Board,
    side: Side,
    piece: Piece,
    attacks_mask_fn: fn(sq: Square) -> u64,
) -> Vec<Move> {
    assert!([Piece::Knight, Piece::King].contains(&piece));
    let mut moves = Vec::new();

    let pieces_bb = board.get_bb(side, piece);

    let opposite_side = side.opposite();

    for from in helpers::get_squares_iter(pieces_bb) {
        let attacks_bb = attacks_mask_fn(from);

        let quiet_moves_bb = attacks_bb & board.get_empty_bb();
        let capture_moves_bb = attacks_bb & board.get_occupancy_bb(opposite_side);

        for to in helpers::get_squares_iter(quiet_moves_bb) {
            let mv = Move::Normal {
                from: from,
                to: to,
                piece: piece,
                captured: None,
                promo: None,
                flags: MoveFlags::empty(),
            };
            moves.push(mv);
        }

        for to in helpers::get_squares_iter(capture_moves_bb) {
            let mv = Move::Normal {
                from: from,
                to: to,
                piece: piece,
                captured: board.get_occupancy_piece(opposite_side, to),
                promo: None,
                flags: MoveFlags::empty(),
            };
            moves.push(mv);
        }
    }

    moves
}

fn generate_sliding_pseudo_legal_moves(
    board: &Board,
    side: Side,
    piece: Piece,
    attacks_mask_fn: fn(sq: Square, occupancy: u64) -> u64,
) -> Vec<Move> {
    let mut moves = Vec::new();

    let piece_bb = board.get_bb(side, piece);
    let opposite_side = side.opposite();

    for from in helpers::get_squares_iter(piece_bb) {
        let attack_bb = attacks_mask_fn(from, board.global_occupancy);

        let quiet_moves_bb = attack_bb & board.get_empty_bb();
        let capture_moves_bb = attack_bb & board.get_occupancy_bb(side.opposite());

        for to in helpers::get_squares_iter(quiet_moves_bb) {
            let mv = Move::Normal {
                from: from,
                to: to,
                piece: piece,
                captured: None,
                promo: None,
                flags: MoveFlags::empty(),
            };
            moves.push(mv);
        }

        for to in helpers::get_squares_iter(capture_moves_bb) {
            let mv = Move::Normal {
                from: from,
                to: to,
                piece: piece,
                captured: board.get_occupancy_piece(opposite_side, to),
                promo: None,
                flags: MoveFlags::empty(),
            };
            moves.push(mv);
        }
    }

    moves
}

fn generate_pseudo_legal_knight_moves(board: &Board, side: Side) -> Vec<Move> {
    generate_leaper_pseudo_legal_moves(board, side, Piece::Knight, get_knight_attacks_mask)
}

fn generate_pseudo_legal_bishop_moves(board: &Board, side: Side) -> Vec<Move> {
    generate_sliding_pseudo_legal_moves(board, side, Piece::Bishop, get_bishop_attacks_mask)
}

fn generate_pseudo_legal_rook_moves(board: &Board, side: Side) -> Vec<Move> {
    generate_sliding_pseudo_legal_moves(board, side, Piece::Rook, get_rook_attacks_mask)
}

fn generate_pseudo_legal_queen_moves(board: &Board, side: Side) -> Vec<Move> {
    generate_sliding_pseudo_legal_moves(board, side, Piece::Queen, get_queen_attacks_mask)
}

fn generate_pseudo_legal_king_moves(board: &Board, side: Side) -> Vec<Move> {
    generate_leaper_pseudo_legal_moves(board, side, Piece::King, get_king_attacks_mask)
}

fn generate_castling_moves(board: &Board, side: Side) -> Vec<Move> {
    let mut moves = Vec::new();

    let castlings = board.game_state.castling_state.get_castlings(side);

    for castling in castlings {
        let (empty_bb, not_attacked_bb) = match (side, castling) {
            (Side::White, CastlingSide::KingSide) => (
                CastlingSide::WHITE_KING_SIDE_EMPTY_MASK,
                CastlingSide::WHITE_KING_SIDE_NOT_ATTACKED_MASK,
            ),
            (Side::White, CastlingSide::QueenSide) => (
                CastlingSide::WHITE_QUEEN_SIDE_EMPTY_MASK,
                CastlingSide::WHITE_QUEEN_SIDE_NOT_ATTACKED_MASK,
            ),
            (Side::Black, CastlingSide::KingSide) => (
                CastlingSide::BLACK_KING_SIDE_EMPTY_MASK,
                CastlingSide::BLACK_KING_SIDE_NOT_ATTACKED_MASK,
            ),
            (Side::Black, CastlingSide::QueenSide) => (
                CastlingSide::BLACK_QUEEN_SIDE_EMPTY_MASK,
                CastlingSide::BLACK_QUEEN_SIDE_NOT_ATTACKED_MASK,
            ),
        };

        let opposite_side = side.opposite();
        if board.global_occupancy & empty_bb == 0
            && helpers::get_squares_iter(not_attacked_bb)
                .all(|square| !board.is_square_attacked(square, opposite_side))
        {
            let mv = Move::Castle { side: castling };
            moves.push(mv);
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

    fn test_pawn_moves(
        moves: &Vec<Move>,
        moves_count: usize,
        double_push_moves_count: usize,
        en_passant_moves_count: usize,
        promo_moves_count: usize,
        contains: &[Move],
    ) {
        assert_eq!(moves.len(), moves_count);
        assert_eq!( moves
                .iter()
                .filter(|&mv| matches!(mv, Move::Normal { flags, .. } if flags.contains(MoveFlags::DOUBLE_PUSH)))
                .count(),
            double_push_moves_count
        );
        assert_eq!(
            moves.iter().filter(
                |&mv| matches!(mv, Move::Normal {flags, ..} if flags.contains(MoveFlags::EN_PASSANT))
            ).count(),
            en_passant_moves_count
        );
        assert_eq!(
            moves
                .iter()
                .filter(|&mv| matches!(mv, Move::Normal {promo, ..} if promo.is_some()))
                .count(),
            promo_moves_count
        );

        for mv in contains {
            assert!(moves.contains(mv));
        }
    }

    #[test]
    fn test_generate_pseudo_legal_pawn_moves_initial_position() {
        let board = Board::get_start_position();
        let white_moves = generate_pseudo_legal_pawn_moves(&board, Side::White);
        let black_moves = generate_pseudo_legal_pawn_moves(&board, Side::Black);

        test_pawn_moves(
            &white_moves,
            16,
            8,
            0,
            0,
            &[
                Move::Normal {
                    from: Square::A2,
                    to: Square::A3,
                    piece: Piece::Pawn,
                    captured: None,
                    promo: None,
                    flags: MoveFlags::empty(),
                },
                Move::Normal {
                    from: Square::A2,
                    to: Square::A4,
                    piece: Piece::Pawn,
                    captured: None,
                    promo: None,
                    flags: MoveFlags::DOUBLE_PUSH,
                },
            ],
        );

        test_pawn_moves(
            &black_moves,
            16,
            8,
            0,
            0,
            &[
                Move::Normal {
                    from: Square::A7,
                    to: Square::A6,
                    piece: Piece::Pawn,
                    captured: None,
                    promo: None,
                    flags: MoveFlags::empty(),
                },
                Move::Normal {
                    from: Square::H7,
                    to: Square::H5,
                    piece: Piece::Pawn,
                    captured: None,
                    promo: None,
                    flags: MoveFlags::DOUBLE_PUSH,
                },
            ],
        );
    }

    #[test]
    fn test_generate_pseudo_legal_pawn_moves_promotion_quiet_and_capture() {
        let board = fen_parser::parse_fen_string("4p3/3P2P1/8/8/8/8/8/8 w - - 0 1").unwrap();
        let white_moves = generate_pseudo_legal_pawn_moves(&board, Side::White);

        test_pawn_moves(
            &white_moves,
            12,
            0,
            0,
            12,
            &[
                Move::Normal {
                    from: Square::G7,
                    to: Square::G8,
                    piece: Piece::Pawn,
                    captured: None,
                    promo: Some(Piece::Queen),
                    flags: MoveFlags::empty(),
                },
                Move::Normal {
                    from: Square::D7,
                    to: Square::E8,
                    piece: Piece::Pawn,
                    captured: Some(Piece::Pawn),
                    promo: Some(Piece::Queen),
                    flags: MoveFlags::empty(),
                },
                Move::Normal {
                    from: Square::D7,
                    to: Square::D8,
                    piece: Piece::Pawn,
                    captured: None,
                    promo: Some(Piece::Knight),
                    flags: MoveFlags::empty(),
                },
            ],
        );
    }

    #[test]
    fn test_generate_pseudo_legal_pawn_moves_captures_and_borders() {
        let board = fen_parser::parse_fen_string("8/8/8/8/2q2p1p/3P2P1/8/8 w - - 0 1").unwrap();
        let white_moves = generate_pseudo_legal_pawn_moves(&board, Side::White);
        let black_moves = generate_pseudo_legal_pawn_moves(&board, Side::Black);

        test_pawn_moves(
            &white_moves,
            5,
            0,
            0,
            0,
            &[
                Move::Normal {
                    from: Square::D3,
                    to: Square::C4,
                    piece: Piece::Pawn,
                    captured: Some(Piece::Queen),
                    promo: None,
                    flags: MoveFlags::empty(),
                },
                Move::Normal {
                    from: Square::G3,
                    to: Square::H4,
                    piece: Piece::Pawn,
                    captured: Some(Piece::Pawn),
                    promo: None,
                    flags: MoveFlags::empty(),
                },
            ],
        );

        test_pawn_moves(
            &black_moves,
            4,
            0,
            0,
            0,
            &[
                Move::Normal {
                    from: Square::H4,
                    to: Square::G3,
                    piece: Piece::Pawn,
                    captured: Some(Piece::Pawn),
                    promo: None,
                    flags: MoveFlags::empty(),
                },
                Move::Normal {
                    from: Square::F4,
                    to: Square::G3,
                    piece: Piece::Pawn,
                    captured: Some(Piece::Pawn),
                    promo: None,
                    flags: MoveFlags::empty(),
                },
            ],
        );
    }

    #[test]
    fn test_generate_pseudo_legal_pawn_moves_en_passant_white() {
        let board = fen_parser::parse_fen_string("8/8/8/Pp1Pp3/8/8/8/8 w - e6 0 1").unwrap();
        let white_moves = generate_pseudo_legal_pawn_moves(&board, Side::White);

        test_pawn_moves(
            &white_moves,
            3,
            0,
            1,
            0,
            &[Move::Normal {
                from: Square::D5,
                to: Square::E6,
                piece: Piece::Pawn,
                captured: Some(Piece::Pawn),
                promo: None,
                flags: MoveFlags::EN_PASSANT,
            }],
        );
    }

    #[test]
    fn test_generate_pseudo_legal_pawn_moves_en_passant_black() {
        let board = fen_parser::parse_fen_string("8/8/8/8/3pP3/8/8/8 b - e3 0 1").unwrap();
        let black_moves = generate_pseudo_legal_pawn_moves(&board, Side::Black);

        test_pawn_moves(
            &black_moves,
            2,
            0,
            1,
            0,
            &[Move::Normal {
                from: Square::D4,
                to: Square::E3,
                piece: Piece::Pawn,
                captured: Some(Piece::Pawn),
                promo: None,
                flags: MoveFlags::EN_PASSANT,
            }],
        );
    }

    #[test]
    fn test_generate_pseudo_legal_pawn_moves_double_push_blocked() {
        let board = fen_parser::parse_fen_string("8/8/8/8/8/4p3/4P3/8 w - - 0 1").unwrap();
        let white_moves = generate_pseudo_legal_pawn_moves(&board, Side::White);
        let black_moves = generate_pseudo_legal_pawn_moves(&board, Side::Black);

        test_pawn_moves(&white_moves, 0, 0, 0, 0, &[]);
        test_pawn_moves(&black_moves, 0, 0, 0, 0, &[]);
    }

    fn test_castling_moves(moves: &[Move], expected_castlings: &[CastlingSide]) {
        if expected_castlings.len() == 0 {
            assert!(!moves.iter().any(|m| matches!(m, Move::Castle { .. })));
        } else {
            assert!(moves
                        .iter()
                        .filter(|m| matches!(m, Move::Castle { .. }))
                        .all(
                            |c_mv| matches!(c_mv, Move::Castle { side } if expected_castlings.contains(&side)),
                        )
            )
        }
    }

    #[test]
    fn test_initial_position_castling_moves() {
        let board = Board::get_start_position();

        let white_moves = generate_castling_moves(&board, Side::White);
        let black_moves = generate_castling_moves(&board, Side::Black);

        test_castling_moves(&white_moves, &[]);
        test_castling_moves(&black_moves, &[]);
    }

    #[test]
    fn test_white_black_king_side_castlings() {
        let board = fen_parser::parse_fen_string("4k2r/8/8/8/8/8/8/4K2R w Kk - 0 1").unwrap();

        let white_moves = generate_castling_moves(&board, Side::White);
        let black_moves = generate_castling_moves(&board, Side::Black);

        test_castling_moves(&white_moves, &[CastlingSide::KingSide]);
        test_castling_moves(&black_moves, &[CastlingSide::KingSide]);
    }

    #[test]
    fn test_white_black_both_side_castlings() {
        let board = fen_parser::parse_fen_string("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();

        let white_moves = generate_castling_moves(&board, Side::White);
        let black_moves = generate_castling_moves(&board, Side::Black);

        test_castling_moves(
            &white_moves,
            &[CastlingSide::KingSide, CastlingSide::QueenSide],
        );
        test_castling_moves(
            &black_moves,
            &[CastlingSide::KingSide, CastlingSide::QueenSide],
        );
    }

    #[test]
    fn test_white_queen_side_castlings_with_different_blockers() {
        // Blockers tests
        let board = fen_parser::parse_fen_string("8/8/8/8/8/8/8/RN2K3 b Q - 0 1").unwrap();
        let white_moves = generate_castling_moves(&board, Side::White);
        test_castling_moves(&white_moves, &[]);

        let board = fen_parser::parse_fen_string("8/8/8/8/8/8/8/R1N1K3 b Q - 0 1").unwrap();
        let white_moves = generate_castling_moves(&board, Side::White);
        test_castling_moves(&white_moves, &[]);

        let board = fen_parser::parse_fen_string("8/8/8/8/8/8/8/R2QK3 b Q - 0 1").unwrap();
        let white_moves = generate_castling_moves(&board, Side::White);
        test_castling_moves(&white_moves, &[]);

        // Attackers test
        let board = fen_parser::parse_fen_string("2r5/8/8/8/8/8/8/R3K3 w Q - 0 1").unwrap();
        let white_moves = generate_castling_moves(&board, Side::White);
        test_castling_moves(&white_moves, &[]);

        let board = fen_parser::parse_fen_string("3r4/8/8/8/8/8/8/R3K3 w Q - 0 1").unwrap();
        let white_moves = generate_castling_moves(&board, Side::White);
        test_castling_moves(&white_moves, &[]);

        let board = fen_parser::parse_fen_string("4r3/8/8/8/8/8/8/R3K3 w Q - 0 1").unwrap();
        let white_moves = generate_castling_moves(&board, Side::White);
        test_castling_moves(&white_moves, &[]);

        let board = fen_parser::parse_fen_string("8/8/7b/8/8/8/8/R3K3 w Q - 0 1").unwrap();
        let white_moves = generate_castling_moves(&board, Side::White);
        test_castling_moves(&white_moves, &[]);

        let board = fen_parser::parse_fen_string("8/8/8/8/8/8/8/R3K3 w - - 0 1").unwrap();
        let white_moves = generate_castling_moves(&board, Side::White);
        test_castling_moves(&white_moves, &[]);
    }

    #[test]
    fn test_white_king_side_castlings_with_different_blockers() {
        // Blockers tests
        let board = fen_parser::parse_fen_string("8/8/8/8/8/8/8/4KN1R b K - 0 1").unwrap();
        let white_moves = generate_castling_moves(&board, Side::White);
        test_castling_moves(&white_moves, &[]);

        let board = fen_parser::parse_fen_string("8/8/8/8/8/8/8/4K1NR b K - 0 1").unwrap();
        let white_moves = generate_castling_moves(&board, Side::White);
        test_castling_moves(&white_moves, &[]);

        // Attackers tests (squares E1/F1/G1 must not be attacked)
        let board = fen_parser::parse_fen_string("5r2/8/8/8/8/8/8/4K2R w K - 0 1").unwrap();
        let white_moves = generate_castling_moves(&board, Side::White);
        test_castling_moves(&white_moves, &[]);

        let board = fen_parser::parse_fen_string("6r1/8/8/8/8/8/8/4K2R w K - 0 1").unwrap();
        let white_moves = generate_castling_moves(&board, Side::White);
        test_castling_moves(&white_moves, &[]);

        let board = fen_parser::parse_fen_string("4r3/8/8/8/8/8/8/4K2R w K - 0 1").unwrap();
        let white_moves = generate_castling_moves(&board, Side::White);
        test_castling_moves(&white_moves, &[]);

        let board = fen_parser::parse_fen_string("8/8/8/1b6/8/8/8/4K2R w K - 0 1").unwrap();
        let white_moves = generate_castling_moves(&board, Side::White);
        test_castling_moves(&white_moves, &[]);

        // No castling rights
        let board = fen_parser::parse_fen_string("8/8/8/8/8/8/8/4K2R w - - 0 1").unwrap();
        let white_moves = generate_castling_moves(&board, Side::White);
        test_castling_moves(&white_moves, &[]);
    }

    #[test]
    fn test_black_king_side_castlings_with_different_blockers() {
        // Blockers tests
        let board = fen_parser::parse_fen_string("4k1nr/8/8/8/8/8/8/8 w k - 0 1").unwrap();
        let black_moves = generate_castling_moves(&board, Side::Black);
        test_castling_moves(&black_moves, &[]);

        let board = fen_parser::parse_fen_string("4kn1r/8/8/8/8/8/8/8 w k - 0 1").unwrap();
        let black_moves = generate_castling_moves(&board, Side::Black);
        test_castling_moves(&black_moves, &[]);

        // Attackers tests (squares E8/F8/G8 must not be attacked)
        let board = fen_parser::parse_fen_string("4k2r/8/8/8/8/8/8/5R2 b k - 0 1").unwrap();
        let black_moves = generate_castling_moves(&board, Side::Black);
        test_castling_moves(&black_moves, &[]);

        let board = fen_parser::parse_fen_string("4k2r/8/8/8/8/8/8/6R1 b k - 0 1").unwrap();
        let black_moves = generate_castling_moves(&board, Side::Black);
        test_castling_moves(&black_moves, &[]);

        let board = fen_parser::parse_fen_string("4k2r/8/8/8/8/8/8/4R3 b k - 0 1").unwrap();
        let black_moves = generate_castling_moves(&board, Side::Black);
        test_castling_moves(&black_moves, &[]);

        let board = fen_parser::parse_fen_string("4k2r/8/8/8/1B6/8/8/8 w k - 0 1").unwrap();
        let black_moves = generate_castling_moves(&board, Side::Black);
        test_castling_moves(&black_moves, &[]);

        // No castling rights
        let board = fen_parser::parse_fen_string("4k2r/8/8/8/8/8/8/8 b - - 0 1").unwrap();
        let black_moves = generate_castling_moves(&board, Side::Black);
        test_castling_moves(&black_moves, &[]);
    }

    #[test]
    fn test_black_queen_side_castlings_with_different_blockers() {
        // Blockers tests
        let board = fen_parser::parse_fen_string("rn2k3/8/8/8/8/8/8/8 w q - 0 1").unwrap();
        let black_moves = generate_castling_moves(&board, Side::Black);
        test_castling_moves(&black_moves, &[]);

        let board = fen_parser::parse_fen_string("r1n1k3/8/8/8/8/8/8/8 w q - 0 1").unwrap();
        let black_moves = generate_castling_moves(&board, Side::Black);
        test_castling_moves(&black_moves, &[]);

        let board = fen_parser::parse_fen_string("r2nk3/8/8/8/8/8/8/8 w q - 0 1").unwrap();
        let black_moves = generate_castling_moves(&board, Side::Black);
        test_castling_moves(&black_moves, &[]);

        // Attackers tests (squares E8/D8/C8 must not be attacked)
        let board = fen_parser::parse_fen_string("r3k3/8/8/8/8/8/8/2R5 b q - 0 1").unwrap();
        let black_moves = generate_castling_moves(&board, Side::Black);
        test_castling_moves(&black_moves, &[]);
        let board = fen_parser::parse_fen_string("r3k3/8/8/8/8/8/8/3R4 b q - 0 1").unwrap();
        let black_moves = generate_castling_moves(&board, Side::Black);
        test_castling_moves(&black_moves, &[]);

        let board = fen_parser::parse_fen_string("r3k3/8/8/8/8/8/8/4R3 b q - 0 1").unwrap();
        let black_moves = generate_castling_moves(&board, Side::Black);
        test_castling_moves(&black_moves, &[]);

        let board = fen_parser::parse_fen_string("r3k3/8/8/8/8/7B/8/8 b q - 0 1").unwrap();
        let black_moves = generate_castling_moves(&board, Side::Black);
        test_castling_moves(&black_moves, &[]);

        // No castling rights
        let board = fen_parser::parse_fen_string("r3k3/8/8/8/8/8/8/8 b - - 0 1").unwrap();
        let black_moves = generate_castling_moves(&board, Side::Black);
        test_castling_moves(&black_moves, &[]);
    }

    #[test]
    fn test_generate_pseudo_legal_moves_initial_position() {
        let board = Board::get_start_position();

        let white_moves = board.generate_pseudo_legal_moves(Side::White);
        let black_moves = board.generate_pseudo_legal_moves(Side::Black);

        assert_eq!(white_moves.len(), 20);
        assert_eq!(black_moves.len(), 20);
    }

    #[test]
    #[ignore]
    fn test_tricky_position_pseudo_legal_move_generation() {
        let board =
            fen_parser::parse_fen_string(chess_consts::fen_strings::TRICKY_POS_FEN).unwrap();

        let white_moves = board.generate_pseudo_legal_moves(Side::White);
        let black_moves = board.generate_pseudo_legal_moves(Side::Black);

        println!("White moves: {}", white_moves.len());
        println!("Black moves: {}", black_moves.len());
    }
}
