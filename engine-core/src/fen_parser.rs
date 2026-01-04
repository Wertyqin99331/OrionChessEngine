use std::fmt::Display;

use crate::{
    board::Board,
    chess_consts,
    enums::{Castling, File, Piece, Rank, Side, Square},
};

const FEN_PARTS_COUNT: usize = 6;
const FEN_PARTS_SPLITTER: char = ' ';
const SIDE_TO_MOVE_CHARS: &str = "wb";

#[derive(Debug)]
pub(crate) enum ParseFenError {
    IncorrectPartsLength,
    PiecesParse,
    SideToMoveParse,
    CastlingRightsParse,
    EnPassantSquareParse,
    HalfMoveClockParse,
    FullMoveCountParse,
}

impl Display for ParseFenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error = match self {
            ParseFenError::IncorrectPartsLength => "Error in FEN string: Must contain 6 parts",
            ParseFenError::PiecesParse => "Error in FEN string: Failed to parse pieces",
            ParseFenError::SideToMoveParse => "Error in FEN string: Failed to parse side to move",
            ParseFenError::CastlingRightsParse => {
                "Error in FEN string: Failed to parse castling rights"
            }
            ParseFenError::EnPassantSquareParse => {
                "Error in FEN string: Failed to parse en-passant square"
            }
            ParseFenError::HalfMoveClockParse => {
                "Error in FEN string: Failed to parse half-moves clock"
            }
            ParseFenError::FullMoveCountParse => {
                "Error in FEN string: Failed to parse full moves count"
            }
        };
        write!(f, "{error}")
    }
}

type ParseFenResult = Result<Board, ParseFenError>;
type ParseFenPartResult = Result<(), ParseFenError>;

pub(crate) fn parse_fen_string(fen: &str) -> ParseFenResult {
    let mut board = Board::default();
    let mut parts: Vec<_> = fen.split(FEN_PARTS_SPLITTER).collect();

    // short fen string case
    if parts.len() == 4 {
        parts.append(&mut vec!["0", "1"]);
    }

    if parts.len() != FEN_PARTS_COUNT {
        return Err(ParseFenError::IncorrectPartsLength);
    }

    let fen_parse_functions = [
        parse_pieces,
        parse_side_to_move,
        parse_castling_rights,
        parse_en_passant_square,
        parse_half_move_clock,
        parse_full_move_number,
    ];

    for (&parse_fn, &part) in fen_parse_functions.iter().zip(parts.iter()) {
        parse_fn(&mut board, part)?;
    }

    Ok(board)
}

fn parse_pieces(board: &mut Board, part: &str) -> ParseFenPartResult {
    let mut rank = Rank::R8.index();
    let mut file = File::A.index();

    for c in part.chars() {
        let mut set_piece = |side: Side, piece: Piece| {
            let square = Square::try_from(rank * chess_consts::BOARD_SIZE as u8 + file)
                .map_err(|_| ParseFenError::PiecesParse)?;
            let square_bb = square.bit();
            *board.get_bb_mut(side, piece) = board.get_bb(side, piece) | square_bb;
            file += 1;
            Ok(())
        };

        match c {
            'K' => set_piece(Side::White, Piece::King)?,
            'Q' => set_piece(Side::White, Piece::Queen)?,
            'R' => set_piece(Side::White, Piece::Rook)?,
            'B' => set_piece(Side::White, Piece::Bishop)?,
            'N' => set_piece(Side::White, Piece::Knight)?,
            'P' => set_piece(Side::White, Piece::Pawn)?,
            'k' => set_piece(Side::Black, Piece::King)?,
            'q' => set_piece(Side::Black, Piece::Queen)?,
            'r' => set_piece(Side::Black, Piece::Rook)?,
            'b' => set_piece(Side::Black, Piece::Bishop)?,
            'n' => set_piece(Side::Black, Piece::Knight)?,
            'p' => set_piece(Side::Black, Piece::Pawn)?,
            '1'..='8' => {
                file += c.to_digit(10).unwrap() as u8;

                if file > chess_consts::BOARD_SIZE as u8 {
                    return Err(ParseFenError::PiecesParse);
                }
            }
            '/' => {
                if file != 8 || rank == 0 {
                    return Err(ParseFenError::PiecesParse);
                }

                rank -= 1;
                file = 0
            }
            _ => return Err(ParseFenError::PiecesParse),
        }
    }

    if file != 8 || rank != Rank::R1.index() {
        return Err(ParseFenError::PiecesParse);
    }

    board.recalc_occupancies();
    Ok(())
}

fn parse_side_to_move(board: &mut Board, part: &str) -> ParseFenPartResult {
    if part.len() == 1
        && let Some(ch) = part.chars().next()
        && SIDE_TO_MOVE_CHARS.contains(ch)
    {
        match ch {
            'w' => board.game_state.side_to_move = Side::White,
            'b' => board.game_state.side_to_move = Side::Black,
            _ => unreachable!(),
        }

        return Ok(());
    }

    return Err(ParseFenError::SideToMoveParse);
}

fn parse_castling_rights(board: &mut Board, part: &str) -> ParseFenPartResult {
    if (1..=4).contains(&part.len()) {
        for ch in part.chars() {
            match ch {
                'K' => board.game_state.castling_state.0 |= Castling::WhiteKingSide.index(),
                'Q' => board.game_state.castling_state.0 |= Castling::WhiteQueenSide.index(),
                'k' => board.game_state.castling_state.0 |= Castling::BlackKingSide.index(),
                'q' => board.game_state.castling_state.0 |= Castling::BlackQueenSide.index(),
                '-' if part.len() == 1 => board.game_state.castling_state.0 = Castling::No.index(),
                _ => return Err(ParseFenError::CastlingRightsParse),
            }
        }

        return Ok(());
    }

    return Err(ParseFenError::CastlingRightsParse);
}

fn parse_en_passant_square(board: &mut Board, part: &str) -> ParseFenPartResult {
    if part.len() == 1
        && let Some(ch) = part.chars().next()
        && ch == '-'
    {
        board.game_state.en_passant_square = None;
        return Ok(());
    }

    if part.len() == 2 {
        let square = part.parse::<Square>();

        match square {
            Ok(sq) if sq.can_be_en_passant() => {
                board.game_state.en_passant_square = Some(sq);
                return Ok(());
            }
            _ => return Err(ParseFenError::EnPassantSquareParse),
        }
    }

    return Err(ParseFenError::EnPassantSquareParse);
}

fn parse_half_move_clock(board: &mut Board, part: &str) -> ParseFenPartResult {
    if (1..=3).contains(&part.len())
        && let Ok(x) = part.parse::<u8>()
        && x <= chess_consts::MAX_HALF_MOVES_COUNT
    {
        board.game_state.half_move_clock = x;
        Ok(())
    } else {
        Err(ParseFenError::HalfMoveClockParse)
    }
}

fn parse_full_move_number(board: &mut Board, part: &str) -> ParseFenPartResult {
    if (1..=5).contains(&part.len())
        && let Ok(x) = part.parse::<u16>()
    {
        board.game_state.full_moves_count = x;
        Ok(())
    } else {
        Err(ParseFenError::FullMoveCountParse)
    }
}

#[cfg(test)]
mod tests {
    use crate::helpers;

    use super::*;

    #[test]
    #[ignore]
    fn test_parse_fen_string() {
        let boards = [
            chess_consts::fen_strings::START_POS_FEN,
            chess_consts::fen_strings::EMPTY_BOARD_FEN,
            chess_consts::fen_strings::TRICKY_POS_FEN,
            chess_consts::fen_strings::KILLER_POS_FEN,
            chess_consts::fen_strings::CMK_POS_FEN,
        ];

        for board in boards {
            match parse_fen_string(board) {
                Ok(b) => {
                    println!("{b}");

                    println!("White occupancies");
                    helpers::print_bitboard(b.get_occupancy_bb(Side::White));

                    println!("Black occupancies");
                    helpers::print_bitboard(b.get_occupancy_bb(Side::Black));

                    println!("Global occupancy");
                    helpers::print_bitboard(b.global_occupancy);

                    println!();
                }
                Err(e) => println!("{e}"),
            }
        }
    }
}
