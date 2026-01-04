use std::mem::MaybeUninit;

use crate::{board::GameState, enums::Move};

const MAX_MOVES_COUNT: usize = 4096;

#[derive(Clone, Debug)]
pub(crate) struct History {
    entries: [MaybeUninit<HistoryEntry>; MAX_MOVES_COUNT],
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
            entries: [const { MaybeUninit::uninit() }; MAX_MOVES_COUNT],
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
