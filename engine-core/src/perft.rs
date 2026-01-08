use crate::{
    board::Board,
    move_generator::{MoveBuffer, MoveGenMode},
};

pub(crate) fn perft(board: &mut Board, depth: u32, ply: usize, bufs: &mut [MoveBuffer]) -> u64 {
    if depth == 0 {
        return 1;
    }

    let (cur, rest) = bufs.split_first_mut().unwrap();

    board.generate_all_legal_moves(board.game_state.side_to_move, cur);

    let mut nodes = 0;

    for &mv in cur.iter() {
        board.make_move(mv);
        nodes += perft(board, depth - 1, ply + 1, rest);
        board.unmake_move();
    }

    nodes
}

#[cfg(test)]
mod tests {
    use crate::{chess_consts, fen_parser};

    use super::*;

    fn test_perft(fen_str: &str, expectations: &[(u32, u64)]) {
        let mut board = fen_parser::parse_fen_string(fen_str).unwrap();

        let mut bufs: Vec<MoveBuffer> = (0..chess_consts::MAX_PLY)
            .map(|_| Vec::with_capacity(chess_consts::MOVES_BUF_SIZE))
            .collect();

        for &(depth, expected_moves_count) in expectations {
            assert_eq!(expected_moves_count, perft(&mut board, depth, 0, &mut bufs));
        }
    }

    #[test]
    fn test_perft_initial_position() {
        test_perft(
            chess_consts::fen_strings::START_POS_FEN,
            &[(1, 20), (2, 400), (3, 8902), (4, 197_281), (5, 4_865_609)],
        );
    }

    #[test]
    fn test_kiwipeter_position() {
        test_perft(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -",
            &[
                (1, 48),
                (2, 2039),
                (3, 97_862),
                (4, 4_085_603),
                (5, 19_3690_690),
                // (6, 8_031_647_685),
            ],
        );
    }

    #[test]
    fn position_3_test() {
        test_perft(
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            &[(1, 14), (2, 191), (3, 2_812), (4, 43_238)],
        );
    }

    #[test]
    fn position_4_test() {
        test_perft(
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            &[(1, 6), (2, 264), (3, 9_467), (4, 422_333), (5, 15_833_292)],
        );
    }

    #[test]
    fn position_5_test() {
        test_perft(
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            &[(1, 44), (2, 1_486), (3, 62_379), (4, 2_103_487)],
        );
    }

    #[test]
    fn position_6_test() {
        test_perft(
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
            &[(1, 46), (2, 2_079), (3, 89_890), (4, 3_894_594)],
        );
    }
}
