use crate::{
    board::Board,
    enums::{CastlingSide, Move, MoveFlags, Piece, Side},
    history::HistoryEntry,
};

impl Board {
    pub(crate) fn make_move(&mut self, mv: Move) {
        // save history
        self.history
            .push(HistoryEntry::new(mv, self.game_state))
            .unwrap();

        let moving_side = self.game_state.side_to_move;
        let opponent_side = moving_side.opposite();

        self.game_state.en_passant_square = None;

        match mv {
            Move::Normal {
                from,
                to,
                piece,
                captured,
                promo,
                flags,
            } => {
                // Remove moving piece from from
                self.remove_piece(moving_side, piece, from);

                // Remove captured piece
                if let Some(captured_piece) = captured {
                    let captured_piece_sq = if flags.contains(MoveFlags::EN_PASSANT) {
                        to.backward(moving_side)
                    } else {
                        to
                    };

                    self.remove_piece(opponent_side, captured_piece, captured_piece_sq);
                }

                // Add moving piece to to
                let piece_to_add = promo.unwrap_or(piece);
                self.add_piece(moving_side, piece_to_add, to);

                // Set en-passant if double-push
                if flags.contains(MoveFlags::DOUBLE_PUSH) {
                    self.game_state.en_passant_square = Some(to.backward(moving_side));
                }

                // Updating castling rights
                if piece == Piece::King {
                    self.game_state.castling_state.remove_all(moving_side);
                }

                if piece == Piece::Rook {
                    self.game_state
                        .castling_state
                        .remove_rook(moving_side, from);
                }

                if let Some(Piece::Rook) = captured {
                    self.game_state
                        .castling_state
                        .remove_rook(opponent_side, to);
                }

                // Update half-move clock
                if piece == Piece::Pawn || captured.is_some() {
                    self.game_state.half_move_clock = 0;
                } else {
                    self.game_state.half_move_clock += 1;
                }
            }
            Move::Castle {
                side: castling_side,
                ..
            } => {
                let (king_from_sq, king_to_sq) =
                    CastlingSide::get_castling_positions(moving_side, Piece::King, castling_side);
                let (rook_from_sq, rook_to_sq) =
                    CastlingSide::get_castling_positions(moving_side, Piece::Rook, castling_side);

                self.move_piece(moving_side, Piece::King, king_from_sq, king_to_sq);
                self.move_piece(moving_side, Piece::Rook, rook_from_sq, rook_to_sq);

                self.game_state.half_move_clock += 1;
                self.game_state.castling_state.remove_all(moving_side);
            }
        }

        if moving_side == Side::Black {
            self.game_state.full_moves_count += 1;
        }

        self.game_state.side_to_move = opponent_side;
    }

    pub(crate) fn unmake_move(&mut self) {
        let HistoryEntry { mv, game_state } = self
            .history
            .pop()
            .expect("Move history was empty while trying to restore state");

        self.game_state = game_state;

        let moving_side = self.game_state.side_to_move;
        let opponent_side = moving_side.opposite();

        match mv {
            Move::Normal {
                from,
                to,
                piece,
                captured,
                promo,
                flags,
            } => {
                let placed_piece = promo.unwrap_or(piece);
                self.remove_piece(moving_side, placed_piece, to);

                self.add_piece(moving_side, piece, from);

                if let Some(captured_piece) = captured {
                    let captured_sq = if flags.contains(MoveFlags::EN_PASSANT) {
                        to.backward(moving_side)
                    } else {
                        to
                    };
                    self.add_piece(opponent_side, captured_piece, captured_sq);
                }
            }
            Move::Castle {
                side: castling_side,
                ..
            } => {
                let (king_from, king_to) =
                    CastlingSide::get_castling_positions(moving_side, Piece::King, castling_side);
                let (rook_from, rook_to) =
                    CastlingSide::get_castling_positions(moving_side, Piece::Rook, castling_side);

                self.remove_piece(moving_side, Piece::King, king_to);
                self.remove_piece(moving_side, Piece::Rook, rook_to);

                self.add_piece(moving_side, Piece::King, king_from);
                self.add_piece(moving_side, Piece::Rook, rook_from);
            }
        }
    }
}
