use crate::{enums::Move, evaluation};

#[derive(Clone, Copy)]
pub(crate) struct TTEntry {
    pub(crate) key: u64,
    pub(crate) depth: u8,
    pub(crate) bound: TTEntryBound,
    pub(crate) score: i16,
    pub(crate) best_move: Option<Move>,
}

impl Default for TTEntry {
    fn default() -> Self {
        Self {
            key: 0u64,
            depth: 0u8,
            bound: TTEntryBound::Exact,
            score: 0i16,
            best_move: None,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum TTEntryBound {
    Exact,
    FailLow,
    FailHigh,
}

pub(crate) struct TranspositionTable {
    entries: Vec<TTEntry>,
    mask: usize,
}

#[allow(dead_code)]
pub(crate) enum ProbeResult {
    Hit {
        score: i32,
        bound: TTEntryBound,
        mv: Option<Move>,
    },
    PartialHit {
        mv: Option<Move>,
    },
    Miss,
}

impl TranspositionTable {
    pub(crate) fn new(mb: usize) -> TranspositionTable {
        let bytes = mb * 1024 * 1024;
        let entry_size = std::mem::size_of::<TTEntry>();
        let entries_count = (bytes / entry_size).next_power_of_two();

        Self {
            entries: vec![TTEntry::default(); entries_count],
            mask: entries_count - 1,
        }
    }

    pub(crate) fn probe(&self, key: u64, depth: u8, ply: u8, alpha: i32, beta: i32) -> ProbeResult {
        let entry = &self.entries[self.index(key)];

        if entry.key != key {
            return ProbeResult::Miss;
        };

        if depth > entry.depth {
            return ProbeResult::PartialHit {
                mv: entry.best_move,
            };
        }
        let score = denormalize_tt_score(entry.score, ply) as i32;

        match entry.bound {
            TTEntryBound::Exact => {
                return ProbeResult::Hit {
                    score: score,
                    bound: TTEntryBound::Exact,
                    mv: entry.best_move,
                };
            }
            TTEntryBound::FailLow if score <= alpha => {
                return ProbeResult::Hit {
                    score: score,
                    bound: TTEntryBound::FailLow,
                    mv: entry.best_move,
                };
            }
            TTEntryBound::FailHigh if score >= beta => {
                return ProbeResult::Hit {
                    score: score,
                    bound: TTEntryBound::FailHigh,
                    mv: entry.best_move,
                };
            }
            _ => ProbeResult::PartialHit {
                mv: entry.best_move,
            },
        }
    }

    pub(crate) fn store(
        &mut self,
        key: u64,
        depth: u8,
        ply: u8,
        score: i16,
        tt_bound: TTEntryBound,
        best_move: Option<Move>,
    ) {
        let idx = self.index(key);
        let entry = &mut self.entries[idx];

        let should_replace = entry.key == 0
            || depth > entry.depth
            || (depth == entry.depth
                && tt_bound == TTEntryBound::Exact
                && entry.bound != TTEntryBound::Exact);

        let score = normalize_tt_score(score, ply);

        if should_replace {
            *entry = TTEntry {
                key: key,
                depth: depth,
                bound: tt_bound,
                score: score,
                best_move: best_move,
            };
        }
    }

    #[inline]
    fn index(&self, key: u64) -> usize {
        key as usize & self.mask
    }
}

#[inline]
fn normalize_tt_score(score: i16, ply: u8) -> i16 {
    if score.abs() > evaluation::MATE_SCORE as i16 {
        if score > 0 {
            return score + ply as i16;
        } else {
            return score - ply as i16;
        }
    }

    return score;
}

#[inline]
fn denormalize_tt_score(score: i16, ply: u8) -> i16 {
    if score.abs() > evaluation::MATE_SCORE as i16 {
        if score > 0 {
            return score - ply as i16;
        } else {
            return score + ply as i16;
        }
    }

    return score;
}
