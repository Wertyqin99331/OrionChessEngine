use std::mem::MaybeUninit;

use crate::{board::GameState, enums::Move};

const MAX_MOVES_COUNT: usize = 512;

#[derive(Clone, Debug)]
pub(crate) struct History {
    entries: Box<[MaybeUninit<HistoryEntry>]>,
    len: usize,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct HistoryEntry {
    pub(crate) mv: Move,
    pub(crate) game_state: GameState,
}

impl HistoryEntry {
    pub(crate) fn new(mv: Move, game_state: GameState) -> HistoryEntry {
        HistoryEntry { mv, game_state }
    }
}

impl History {
    pub(crate) fn new() -> History {
        History {
            entries: vec![MaybeUninit::uninit(); MAX_MOVES_COUNT].into_boxed_slice(),
            len: 0,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn push(&mut self, entry: HistoryEntry) -> Result<(), HistoryEntry> {
        if self.len == MAX_MOVES_COUNT {
            return Err(entry);
        }

        self.entries[self.len].write(entry);
        self.len += 1;
        Ok(())
    }

    pub(crate) fn pop(&mut self) -> Option<HistoryEntry> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;
        unsafe { Some(self.entries[self.len].assume_init_read()) }
    }

    pub(crate) fn get_repetition_count(&self, zobrist_key: u64, half_moves: u8) -> u8 {
        if self.len == 0 {
            return 0;
        }

        let mut count = 0;
        let mut offset = 2;

        let max_offset = self.len.min(half_moves as usize);
        let len = self.len;

        while offset <= max_offset {
            unsafe {
                if self.entries[len - offset]
                    .assume_init_ref()
                    .game_state
                    .zobrist_key
                    == zobrist_key
                {
                    count += 1;
                }
            }

            offset += 2;
        }

        count
    }
}

impl Default for History {
    fn default() -> Self {
        History::new()
    }
}

impl Drop for History {
    fn drop(&mut self) {
        for i in 0..self.len {
            unsafe { self.entries[i].assume_init_drop() }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        board::Board,
        enums::{MoveFlags, Piece, Square},
        init,
    };

    use super::*;

    #[test]
    fn test_repetition_detection() {
        init::init_engine();

        let mut board = Board::get_start_position();

        board.make_move(Move::Normal {
            from: Square::D2,
            to: Square::D4,
            piece: Piece::Pawn,
            captured: None,
            promo: None,
            flags: MoveFlags::DOUBLE_PUSH,
        });

        board.make_move(Move::Normal {
            from: Square::D7,
            to: Square::D5,
            piece: Piece::Pawn,
            captured: None,
            promo: None,
            flags: MoveFlags::DOUBLE_PUSH,
        });

        for _ in 0..4 {
            board.make_move(Move::Normal {
                from: Square::D1,
                to: Square::D2,
                piece: Piece::Queen,
                captured: None,
                promo: None,
                flags: MoveFlags::empty(),
            });
            board.make_move(Move::Normal {
                from: Square::D8,
                to: Square::D7,
                piece: Piece::Queen,
                captured: None,
                promo: None,
                flags: MoveFlags::empty(),
            });
        }

        assert_eq!(
            board.history.get_repetition_count(
                board.game_state.zobrist_key,
                board.game_state.half_move_clock
            ),
            2
        );
    }
}
