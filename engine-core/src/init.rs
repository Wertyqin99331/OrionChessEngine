use std::sync::Once;

use crate::{sliding_piece_attack_table, zobrist_hashing};

static INIT: Once = Once::new();

pub(crate) fn init_engine() {
    INIT.call_once(|| {
        sliding_piece_attack_table::init_bishop_magics_attacks();
        sliding_piece_attack_table::init_rook_magics_attacks();
        zobrist_hashing::init_zobrist_hash_keys();
    });
}
