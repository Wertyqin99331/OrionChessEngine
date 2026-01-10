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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MoveGenMode {
    All,
    CapturesOnly,
}

pub(crate) type MoveBuffer = Vec<Move>;

impl Board {
    pub(crate) fn generate_pseudo_legal_moves(
        &self,
        mode: MoveGenMode,
        side: Side,
        buf: &mut MoveBuffer,
    ) {
        buf.clear();

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
            handler(self, mode, side, buf);
        }
    }

    pub(crate) fn generate_legal_moves(
        &mut self,
        mode: MoveGenMode,
        side: Side,
        buf: &mut MoveBuffer,
    ) {
        self.generate_pseudo_legal_moves(mode, side, buf);

        let mut write = 0;
        let buf_len = buf.len();

        for read in 0..buf_len {
            let mv = buf[read];

            self.make_move(mv);
            let ok = !self.is_in_check(side);
            self.unmake_move();

            if ok {
                buf[write] = mv;
                write += 1;
            }
        }

        buf.truncate(write);
    }

    pub(crate) fn generate_all_legal_moves(&mut self, side: Side, buf: &mut MoveBuffer) {
        self.generate_legal_moves(MoveGenMode::All, side, buf);
    }

    pub(crate) fn generate_legal_captures(&mut self, side: Side, buf: &mut MoveBuffer) {
        self.generate_legal_moves(MoveGenMode::CapturesOnly, side, buf);
    }

    pub(crate) fn generate_all_legal_moves_to_vec(&mut self, side: Side) -> Vec<Move> {
        let mut buf = Vec::with_capacity(chess_consts::MOVES_BUF_SIZE);

        self.generate_all_legal_moves(side, &mut buf);

        buf
    }

    #[allow(dead_code)]
    pub(crate) fn generate_legal_captures_to_vec(&mut self, side: Side) -> Vec<Move> {
        let mut buf = Vec::with_capacity(chess_consts::MOVES_BUF_SIZE);

        self.generate_legal_captures(side, &mut buf);

        buf
    }
}

fn generate_pseudo_legal_pawn_moves(
    board: &Board,
    mode: MoveGenMode,
    side: Side,
    buf: &mut Vec<Move>,
) {
    let pawn_bb = board.get_bb(side, Piece::Pawn);

    if mode == MoveGenMode::All {
        // Generate pawn moves
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

            buf.push(Move::Normal {
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
                buf.push(mv);
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
            buf.push(mv);
        }
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
                    buf.push(mv);
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
                buf.push(mv);
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
                buf.push(mv);
            }
        }
    }
}

fn generate_leaper_pseudo_legal_moves(
    board: &Board,
    mode: MoveGenMode,
    side: Side,
    piece: Piece,
    attacks_mask_fn: fn(sq: Square) -> u64,
    buf: &mut MoveBuffer,
) {
    let pieces_bb = board.get_bb(side, piece);

    let opposite_side = side.opposite();

    for from in helpers::get_squares_iter(pieces_bb) {
        let attacks_bb = attacks_mask_fn(from);

        if mode == MoveGenMode::All {
            let quiet_moves_bb = attacks_bb & board.get_empty_bb();

            for to in helpers::get_squares_iter(quiet_moves_bb) {
                let mv = Move::Normal {
                    from: from,
                    to: to,
                    piece: piece,
                    captured: None,
                    promo: None,
                    flags: MoveFlags::empty(),
                };
                buf.push(mv);
            }
        }

        let capture_moves_bb = attacks_bb & board.get_occupancy_bb(opposite_side);

        for to in helpers::get_squares_iter(capture_moves_bb) {
            let mv = Move::Normal {
                from: from,
                to: to,
                piece: piece,
                captured: board.get_occupancy_piece(opposite_side, to),
                promo: None,
                flags: MoveFlags::empty(),
            };
            buf.push(mv);
        }
    }
}

fn generate_sliding_pseudo_legal_moves(
    board: &Board,
    mode: MoveGenMode,
    side: Side,
    piece: Piece,
    attacks_mask_fn: fn(sq: Square, occupancy: u64) -> u64,
    buf: &mut MoveBuffer,
) {
    let piece_bb = board.get_bb(side, piece);
    let opposite_side = side.opposite();

    for from in helpers::get_squares_iter(piece_bb) {
        let attack_bb = attacks_mask_fn(from, board.global_occupancy);

        if mode == MoveGenMode::All {
            let quiet_moves_bb = attack_bb & board.get_empty_bb();

            for to in helpers::get_squares_iter(quiet_moves_bb) {
                let mv = Move::Normal {
                    from: from,
                    to: to,
                    piece: piece,
                    captured: None,
                    promo: None,
                    flags: MoveFlags::empty(),
                };
                buf.push(mv);
            }
        }

        let capture_moves_bb = attack_bb & board.get_occupancy_bb(side.opposite());

        for to in helpers::get_squares_iter(capture_moves_bb) {
            let mv = Move::Normal {
                from: from,
                to: to,
                piece: piece,
                captured: board.get_occupancy_piece(opposite_side, to),
                promo: None,
                flags: MoveFlags::empty(),
            };
            buf.push(mv);
        }
    }
}

fn generate_pseudo_legal_knight_moves(
    board: &Board,
    mode: MoveGenMode,
    side: Side,
    buf: &mut MoveBuffer,
) {
    generate_leaper_pseudo_legal_moves(
        board,
        mode,
        side,
        Piece::Knight,
        get_knight_attacks_mask,
        buf,
    )
}

fn generate_pseudo_legal_bishop_moves(
    board: &Board,
    mode: MoveGenMode,
    side: Side,
    buf: &mut MoveBuffer,
) {
    generate_sliding_pseudo_legal_moves(
        board,
        mode,
        side,
        Piece::Bishop,
        get_bishop_attacks_mask,
        buf,
    )
}

fn generate_pseudo_legal_rook_moves(
    board: &Board,
    mode: MoveGenMode,
    side: Side,
    buf: &mut MoveBuffer,
) {
    generate_sliding_pseudo_legal_moves(board, mode, side, Piece::Rook, get_rook_attacks_mask, buf)
}

fn generate_pseudo_legal_queen_moves(
    board: &Board,
    mode: MoveGenMode,
    side: Side,
    buf: &mut MoveBuffer,
) {
    generate_sliding_pseudo_legal_moves(
        board,
        mode,
        side,
        Piece::Queen,
        get_queen_attacks_mask,
        buf,
    )
}

fn generate_pseudo_legal_king_moves(
    board: &Board,
    mode: MoveGenMode,
    side: Side,
    buf: &mut MoveBuffer,
) {
    generate_leaper_pseudo_legal_moves(board, mode, side, Piece::King, get_king_attacks_mask, buf)
}

fn generate_castling_moves(board: &Board, _: MoveGenMode, side: Side, buf: &mut MoveBuffer) {
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
            buf.push(mv);
        }
    }
}

#[inline(always)]
fn push_pawn(bb: u64, side: Side) -> u64 {
    if side == Side::White {
        bb << chess_consts::BOARD_SIZE
    } else {
        bb >> chess_consts::BOARD_SIZE
    }
}
