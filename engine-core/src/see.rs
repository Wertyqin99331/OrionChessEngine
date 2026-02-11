use crate::{
    board::Board,
    enums::{Move, Piece, Side, Square},
    evaluation,
    helpers::{self, get_attacks_mask},
};

pub(crate) const SEE_DEPTH: u32 = 4;

struct AttackerInfo {
    piece: Piece,
    from: Square,
}

impl Board {
    fn get_least_valuable_attacker(
        &self,
        side: Side,
        target_sq: Square,
        actual_occupancy: u64,
    ) -> Option<AttackerInfo> {
        let pieces_by_asc = [
            Piece::Pawn,
            Piece::Knight,
            Piece::Bishop,
            Piece::Rook,
            Piece::Queen,
            Piece::King,
        ];

        for piece in pieces_by_asc {
            let candidates_bb =
                get_attacks_mask(side.opposite(), piece, target_sq, actual_occupancy);

            let attackers_bb = self.get_bb(side, piece) & actual_occupancy & candidates_bb;
            if attackers_bb != 0 {
                let first_sq = helpers::get_squares_iter(attackers_bb).next().unwrap();
                return Some(AttackerInfo {
                    piece: piece,
                    from: first_sq,
                });
            }
        }

        None
    }

    pub(crate) fn see(&self, mv: Move) -> i32 {
        if !mv.is_capture() {
            panic!("See can be called only with a capture move");
        }

        let (from, to, piece, captured) = match mv {
            Move::Normal {
                from,
                to,
                piece,
                captured,
                ..
            } => (from, to, piece, captured.unwrap()),
            _ => return 0,
        };

        let mut gains = [0; 32];
        gains[0] = evaluation::get_piece_value(captured);
        let mut gain_index = 1;
        let mut side_to_move = self.game_state.side_to_move;

        let mut occupancy = self.global_occupancy;
        occupancy = helpers::pop_bit(occupancy, from);
        occupancy = helpers::pop_bit(occupancy, mv.get_captured_piece_sq(side_to_move).unwrap());
        occupancy = helpers::set_bit(occupancy, to);

        let mut next_victim_value = evaluation::get_piece_value(piece);

        loop {
            side_to_move = side_to_move.opposite();

            let next_attacker_info = self.get_least_valuable_attacker(side_to_move, to, occupancy);
            if next_attacker_info.is_none() {
                break;
            }

            let next_attacker_info = next_attacker_info.unwrap();
            occupancy = helpers::pop_bit(occupancy, next_attacker_info.from);

            if next_attacker_info.piece == Piece::King
                && self
                    .get_least_valuable_attacker(side_to_move.opposite(), to, occupancy)
                    .is_some()
            {
                break;
            }

            gains[gain_index] = next_victim_value;
            gain_index += 1;

            next_victim_value = evaluation::get_piece_value(next_attacker_info.piece);
        }

        gain_index -= 1;
        while gain_index >= 1 {
            gains[gain_index - 1] = (gains[gain_index - 1] - gains[gain_index]).max(0);
            gain_index -= 1;
        }

        gains[0]
    }
}
