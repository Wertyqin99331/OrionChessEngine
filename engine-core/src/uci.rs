use crate::{
    board::Board,
    enums::{CastlingSide, Move, Piece, Side},
    fen_parser,
    move_generator::MoveGenMode,
};

pub(crate) fn serialize_move_to_uci_str(mv: Move, side: Side) -> String {
    match mv {
        Move::Normal {
            from, to, promo, ..
        } => {
            let mut mv_str = format!("{}{}", from, to);

            if let Some(promo_piece) = promo {
                let promo_cg = match promo_piece {
                    Piece::Knight => 'n',
                    Piece::Bishop => 'b',
                    Piece::Rook => 'r',
                    Piece::Queen => 'q',
                    _ => unreachable!(),
                };
                mv_str.push(promo_cg);
            }
            return mv_str;
        }
        Move::Castle {
            side: castling_side,
        } => {
            let (from, to) = CastlingSide::get_castling_positions(side, Piece::King, castling_side);
            let mv_str = format!("{from}{to}");
            return mv_str;
        }
    }
}

pub(crate) fn parse_uci_move(move_str: &str, board: &mut Board) -> Option<Move> {
    let moving_side = board.game_state.side_to_move;
    let moves = board.generate_all_legal_moves_to_vec(moving_side);

    for mv in moves {
        if move_str == &serialize_move_to_uci_str(mv, moving_side) {
            return Some(mv);
        }
    }

    None
}

pub fn parse_uci_position_command(position_str: &str) -> Result<Board, &'static str> {
    let parts: Vec<_> = position_str.split_whitespace().collect();

    if [0, 1].contains(&parts.len()) || parts[0] != "position" {
        return Err("The string is not a valid position command");
    }

    let (mut board, moves_index) = if parts[1] == "startpos" {
        (Board::get_start_position(), 2)
    } else if parts[1] == "fen" {
        if parts.len() < 8 {
            return Err("The fen position was incorrect");
        }

        let fen_str = parts[2..=7].join(" ");
        (
            fen_parser::parse_fen_string(&fen_str)
                .map_err(|_| "An error occured during parsing the fen string")?,
            8,
        )
    } else {
        return Err("The string is not a valid position command");
    };

    if parts.len() == moves_index {
        return Ok(board);
    }

    if !(parts[moves_index] == "moves") {
        return Err("The string is not a valid position command");
    }

    if parts.len() == moves_index + 1 {
        return Ok(board);
    }

    for &mv in &parts[moves_index + 1..] {
        if let Some(mv) = parse_uci_move(mv, &mut board) {
            board.make_move(mv);
        } else {
            return Err("The move in the move section was invalid");
        }
    }

    Ok(board)
}

pub(crate) fn parse_uci_go_commmand(command: &str) -> Result<UciGoCommand, &'static str> {
    let error = "The string is not a valid go command";
    let parts: Vec<_> = command.split_whitespace().collect();

    if parts.len() == 0 {
        return Err(error);
    }

    if parts.len() == 1 {
        return Ok(UciGoCommand {
            mode: GoMode::Infinite,
            tc: TimeControl::default(),
            search_moves: None,
            nodes: None,
            mate: None,
        });
    }

    match parts[1] {
        "depth" => {
            if parts.len() < 3 {
                return Err(error);
            }

            let depth = parts[2]
                .parse::<u32>()
                .map_err(|_| "Failed to parse depth")?;
            return Ok(UciGoCommand {
                mode: GoMode::Depth(depth),
                tc: TimeControl::default(),
                search_moves: None,
                nodes: None,
                mate: None,
            });
        }
        "movetime" => {
            if parts.len() < 3 {
                return Err(error);
            }
            let search_time = parts[2]
                .parse::<u64>()
                .map_err(|_| "Failed to parse search time")?;

            return Ok(UciGoCommand {
                mode: GoMode::MoveTime(search_time),
                tc: TimeControl::default(),
                search_moves: None,
                nodes: None,
                mate: None,
            });
        }
        "infinite" => {
            return Ok(UciGoCommand {
                mode: GoMode::Infinite,
                tc: TimeControl::default(),
                search_moves: None,
                nodes: None,
                mate: None,
            });
        }
        _ => Ok(UciGoCommand {
            mode: GoMode::Infinite,
            tc: TimeControl::default(),
            search_moves: None,
            nodes: None,
            mate: None,
        }),
    }
}

#[derive(Debug, Clone)]
pub struct UciGoCommand {
    pub mode: GoMode,
    pub tc: TimeControl,
    pub search_moves: Option<Vec<Move>>,
    pub nodes: Option<u64>,
    pub mate: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GoMode {
    Depth(u32),
    MoveTime(u64),
    Infinite,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TimeControl {
    pub wtime: Option<u64>,
    pub btime: Option<u64>,
    pub winc: Option<u64>,
    pub binc: Option<u64>,
}

#[cfg(test)]
mod tests {
    use crate::{
        enums::{MoveFlags, Square},
        fen_parser,
    };

    use super::*;

    #[test]
    fn test_normal_and_promo_move_serialization() {
        let mv = Move::Normal {
            from: Square::A2,
            to: Square::A4,
            piece: Piece::Pawn,
            captured: None,
            promo: None,
            flags: MoveFlags::empty(),
        };
        assert_eq!("a2a4", serialize_move_to_uci_str(mv, Side::White));

        let mv = Move::Normal {
            from: Square::A7,
            to: Square::A8,
            piece: Piece::Pawn,
            captured: None,
            promo: Some(Piece::Queen),
            flags: MoveFlags::empty(),
        };
        assert_eq!("a7a8q", serialize_move_to_uci_str(mv, Side::White));

        let mv = Move::Normal {
            from: Square::A7,
            to: Square::A5,
            piece: Piece::Pawn,
            captured: None,
            promo: None,
            flags: MoveFlags::empty(),
        };
        assert_eq!("a7a5", serialize_move_to_uci_str(mv, Side::White));

        let mv = Move::Normal {
            from: Square::A2,
            to: Square::A1,
            piece: Piece::Pawn,
            captured: None,
            promo: Some(Piece::Rook),
            flags: MoveFlags::empty(),
        };
        assert_eq!("a2a1r", serialize_move_to_uci_str(mv, Side::White));
    }

    #[test]
    fn test_castling_moves_serialization() {
        let king_side_castle = Move::Castle {
            side: CastlingSide::KingSide,
        };

        assert_eq!(
            "e1g1",
            serialize_move_to_uci_str(king_side_castle, Side::White)
        );
        assert_eq!(
            "e8g8",
            serialize_move_to_uci_str(king_side_castle, Side::Black)
        );

        let queen_side_castle = Move::Castle {
            side: CastlingSide::QueenSide,
        };

        assert_eq!(
            "e1c1",
            serialize_move_to_uci_str(queen_side_castle, Side::White)
        );
        assert_eq!(
            "e8c8",
            serialize_move_to_uci_str(queen_side_castle, Side::Black)
        );
    }

    #[test]
    fn test_parsing_moves_normal_promo_moves() {
        let mut board = Board::get_start_position();

        let mv = parse_uci_move("a2a3", &mut board);
        assert_eq!(
            mv,
            Some(Move::Normal {
                from: Square::A2,
                to: Square::A3,
                piece: Piece::Pawn,
                captured: None,
                promo: None,
                flags: MoveFlags::empty()
            })
        );

        let mv = parse_uci_move("a2a4", &mut board);
        assert_eq!(
            mv,
            Some(Move::Normal {
                from: Square::A2,
                to: Square::A4,
                piece: Piece::Pawn,
                captured: None,
                promo: None,
                flags: MoveFlags::DOUBLE_PUSH
            })
        );

        let mv = parse_uci_move("b1c3", &mut board);
        assert_eq!(
            mv,
            Some(Move::Normal {
                from: Square::B1,
                to: Square::C3,
                piece: Piece::Knight,
                captured: None,
                promo: None,
                flags: MoveFlags::empty()
            })
        );

        let mut board = fen_parser::parse_fen_string("2q5/1P6/8/8/8/8/8/K7 w - - 0 1").unwrap();

        let mv = parse_uci_move("b7b8q", &mut board);
        assert_eq!(
            mv,
            Some(Move::Normal {
                from: Square::B7,
                to: Square::B8,
                piece: Piece::Pawn,
                captured: None,
                promo: Some(Piece::Queen),
                flags: MoveFlags::empty()
            })
        );

        let mv = parse_uci_move("b7c8n", &mut board);
        assert_eq!(
            mv,
            Some(Move::Normal {
                from: Square::B7,
                to: Square::C8,
                piece: Piece::Pawn,
                captured: Some(Piece::Queen),
                promo: Some(Piece::Knight),
                flags: MoveFlags::empty()
            })
        );

        let mut board = fen_parser::parse_fen_string("2q4k/p7/8/8/8/8/6p1/5R2 b - - 0 1").unwrap();

        let mv = parse_uci_move("g2g1b", &mut board);
        assert_eq!(
            mv,
            Some(Move::Normal {
                from: Square::G2,
                to: Square::G1,
                piece: Piece::Pawn,
                captured: None,
                promo: Some(Piece::Bishop),
                flags: MoveFlags::empty()
            })
        );

        let mv = parse_uci_move("g2f1q", &mut board);
        assert_eq!(
            mv,
            Some(Move::Normal {
                from: Square::G2,
                to: Square::F1,
                piece: Piece::Pawn,
                captured: Some(Piece::Rook),
                promo: Some(Piece::Queen),
                flags: MoveFlags::empty()
            })
        );

        let mv = parse_uci_move("c8a8", &mut board);
        assert_eq!(
            mv,
            Some(Move::Normal {
                from: Square::C8,
                to: Square::A8,
                piece: Piece::Queen,
                captured: None,
                promo: None,
                flags: MoveFlags::empty()
            })
        );
    }

    #[test]
    fn test_parse_castling_moves() {
        let mut board = fen_parser::parse_fen_string("8/8/8/8/8/8/8/R3K2R w KQ - 0 1").unwrap();

        let mv = parse_uci_move("e1g1", &mut board);
        assert_eq!(
            mv,
            Some(Move::Castle {
                side: CastlingSide::KingSide
            })
        );

        let mv = parse_uci_move("e1c1", &mut board);
        assert_eq!(
            mv,
            Some(Move::Castle {
                side: CastlingSide::QueenSide
            })
        );

        let mut board = fen_parser::parse_fen_string("r3k2r/8/8/8/8/8/8/8 b kq - 0 1").unwrap();

        let mv = parse_uci_move("e8g8", &mut board);
        assert_eq!(
            mv,
            Some(Move::Castle {
                side: CastlingSide::KingSide
            })
        );

        let mv = parse_uci_move("e8c8", &mut board);
        assert_eq!(
            mv,
            Some(Move::Castle {
                side: CastlingSide::QueenSide
            })
        );
    }

    #[test]
    fn test_parse_position_function() {
        assert!(matches!(
            parse_uci_position_command("position startpos"),
            Ok(_)
        ));
        assert!(
            matches!(parse_uci_position_command("position startpos moves e2e4"), Ok(board) if board.history.len() == 1 && board.game_state.side_to_move == Side::Black)
        );
        assert!(
            matches!(parse_uci_position_command("position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"), Ok(board) if board.game_state.side_to_move == Side::White && board.game_state.full_moves_count == 1)
        );
        assert!(
            matches!(parse_uci_position_command("position fen rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1 moves c7c5"), Ok(board) if board.history.len() == 1)
        );

        assert!(
            matches!(parse_uci_position_command("position startpos moves"), Ok(board) if board.history.len() == 0)
        );
        assert!(
            matches!(parse_uci_position_command("position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 moves"), Ok(board) if board.history.len() == 0)
        );
        assert!(
            matches!(parse_uci_position_command("position startpos moves e2e4 e7e5"), Ok(board) if board.history.len() == 2)
        );

        assert!(matches!(parse_uci_position_command("position"), Err(_)));
        assert!(matches!(
            parse_uci_position_command(
                "position startpos fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
            ),
            Err(_)
        ));
        assert!(matches!(
            parse_uci_position_command("position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR"),
            Err(_)
        ));
        assert!(matches!(
            parse_uci_position_command(
                "position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 extra"
            ),
            Err(_)
        ));
        assert!(matches!(
            parse_uci_position_command("position startpos moves e4"),
            Err(_)
        ));
    }

    #[test]
    fn test_parse_uci_go_command() {
        assert!(parse_uci_go_commmand("go").is_ok());
        assert!(matches!(
            parse_uci_go_commmand("go depth 3"),
            Ok(UciGoCommand {
                mode: GoMode::Depth(_),
                ..
            })
        ));
        assert!(matches!(
            parse_uci_go_commmand("go movetime 10000"),
            Ok(UciGoCommand {
                mode: GoMode::MoveTime(_),
                ..
            })
        ));
        assert!(matches!(
            parse_uci_go_commmand("go infinite"),
            Ok(UciGoCommand {
                mode: GoMode::Infinite,
                ..
            })
        ))
    }
}
