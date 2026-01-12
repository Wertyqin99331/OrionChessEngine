use bitflags;
use std::fmt;

use crate::chess_consts;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Side {
    White,
    Black,
}

impl Default for Side {
    fn default() -> Self {
        Side::White
    }
}

impl Side {
    #[inline]
    pub(crate) const fn index(self) -> u8 {
        self as u8
    }

    #[inline]
    pub(crate) const fn opposite(self) -> Side {
        match self {
            Self::White => Side::Black,
            Self::Black => Side::White,
        }
    }

    #[inline]
    pub(crate) const unsafe fn from_u8_unchecked(v: u8) -> Side {
        unsafe { std::mem::transmute(v) }
    }

    pub(crate) fn all() -> impl Iterator<Item = Side> {
        (0..2).map(|v| unsafe { Side::from_u8_unchecked(v) })
    }

    pub(crate) fn get_promotion_rank(self) -> Rank {
        match self {
            Side::White => Rank::R8,
            Side::Black => Rank::R1,
        }
    }
}

impl Into<u8> for Side {
    fn into(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for Side {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == 0 {
            Ok(Side::White)
        } else if value == 1 {
            Ok(Side::Black)
        } else {
            Err(())
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[allow(dead_code)]
#[rustfmt::skip]
pub(crate) enum Square {
    A1, B1, C1, D1, E1, F1, G1, H1,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A8, B8, C8, D8, E8, F8, G8, H8,
}

impl Square {
    #[inline]
    pub(crate) const fn index(self) -> u8 {
        self as u8
    }

    #[inline]
    pub(crate) const fn bit(self) -> u64 {
        1u64 << (self as u64)
    }

    #[inline]
    pub(crate) const fn rank(self) -> Rank {
        unsafe { Rank::from_u8_unchecked(self.index() / 8) }
    }

    #[inline]
    pub(crate) const fn file(self) -> Rank {
        unsafe { Rank::from_u8_unchecked(self.index() % 8) }
    }

    #[inline]
    pub(crate) const unsafe fn from_u8_unchecked(v: u8) -> Square {
        unsafe { std::mem::transmute(v) }
    }

    pub(crate) fn all() -> impl Iterator<Item = Square> {
        Square::range(Square::A1, Square::H8)
    }

    pub(crate) fn range(from: Square, to: Square) -> impl Iterator<Item = Square> {
        (from.index()..=to.index()).map(|v| unsafe { Square::from_u8_unchecked(v) })
    }

    #[inline]
    pub(crate) fn can_be_en_passant(self) -> bool {
        (Square::A3.index()..=Square::H3.index()).contains(&self.index())
            || (Square::A6.index()..=Square::H6.index()).contains(&self.index())
    }

    /// Checks wheter the given square can be target for given sisde
    /// # Examples
    /// A6 White - true, A6 can be captured by white paswn as en-passant square
    /// A6 Black - false, all black capture en-passant squares are on the 3th rank
    /// A3 Black - true
    /// A3 White - false
    pub(crate) fn is_en_passant_target_for(self, side: Side) -> bool {
        match side {
            Side::White => self.rank() == Rank::R6,
            Side::Black => self.rank() == Rank::R3,
        }
    }

    /// Moves backward on one rank
    /// # Examples
    /// A2 White -> A1
    /// A2 Black -> A3
    pub(crate) fn backward(self, side: Side) -> Square {
        match side {
            Side::White => unsafe {
                Square::from_u8_unchecked(self.index() - chess_consts::BOARD_SIZE as u8)
            },
            Side::Black => unsafe {
                Square::from_u8_unchecked(self.index() + chess_consts::BOARD_SIZE as u8)
            },
        }
    }
}

impl Into<u8> for Square {
    fn into(self) -> u8 {
        self.index()
    }
}

impl TryFrom<u8> for Square {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < 64 {
            Ok(unsafe { std::mem::transmute(value) })
        } else {
            Err(())
        }
    }
}

impl TryFrom<&str> for Square {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let bytes = s.as_bytes();
        if bytes.len() != 2 {
            return Err(());
        }

        let file = bytes[0];
        let rank = bytes[1];

        let file = match file {
            b'a'..=b'h' => file - b'a',
            b'A'..=b'H' => file - b'A',
            _ => return Err(()),
        };

        let rank = match rank {
            b'1'..=b'8' => rank - b'1',
            _ => return Err(()),
        };

        let idx = rank * 8 + file;
        Ok(unsafe { Square::from_u8_unchecked(idx) })
    }
}

impl std::str::FromStr for Square {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Square::try_from(s)
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let idx = *self as u8;
        let file = (b'a' + (idx % 8)) as char;
        let rank = (idx / 8) + 1;
        write!(f, "{file}{rank}")
    }
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq)]
#[allow(dead_code)]
#[rustfmt::skip]
pub(crate) enum File { A=0, B=1, C=2, D=3, E=4, F=5, G=6, H=7 }

impl File {
    pub(crate) const fn index(self) -> u8 {
        self as u8
    }

    #[allow(dead_code)]
    pub(crate) const unsafe fn from_u8_unchecked(value: u8) -> Rank {
        unsafe { std::mem::transmute(value) }
    }
}

impl TryFrom<u8> for File {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < 8 {
            Ok(unsafe { std::mem::transmute(value) })
        } else {
            Err(())
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq)]
#[allow(dead_code)]
#[rustfmt::skip]
pub(crate) enum Rank { R1=0, R2=1, R3=2, R4=3, R5=4, R6=5, R7=6, R8=7 }

impl Rank {
    pub(crate) const fn index(self) -> u8 {
        self as u8
    }

    pub(crate) const unsafe fn from_u8_unchecked(value: u8) -> Rank {
        unsafe { std::mem::transmute(value) }
    }
}

impl TryFrom<u8> for Rank {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < 8 {
            Ok(unsafe { std::mem::transmute(value) })
        } else {
            Err(())
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[rustfmt::skip]
pub(crate) enum Piece {Pawn, Knight, Bishop, Rook, Queen, King}

impl Piece {
    pub(crate) const PROMOTION_PIECES: [Piece; 4] =
        [Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen];

    pub(crate) const fn index(self) -> u8 {
        self as u8
    }

    pub(crate) unsafe fn from_u8_unchecked(value: u8) -> Piece {
        unsafe { std::mem::transmute(value) }
    }

    pub(crate) fn all() -> impl Iterator<Item = Piece> {
        (0..6).map(|v| unsafe { Piece::from_u8_unchecked(v) })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Move {
    Normal {
        from: Square,
        to: Square,
        piece: Piece,
        captured: Option<Piece>,
        promo: Option<Piece>,
        flags: MoveFlags,
    },
    Castle {
        from: Square,
        to: Square,
        side: CastlingSide,
    },
}

impl Move {
    pub(crate) fn is_capture(&self) -> bool {
        matches!(
            self,
            Move::Normal {
                captured: Some(_),
                ..
            }
        )
    }

    pub(crate) fn is_promo(&self) -> bool {
        matches!(self, Move::Normal { promo: Some(_), .. })
    }

    pub(crate) fn get_castling_move(side: Side, castling_side: CastlingSide) -> Move {
        let (from, to) = CastlingSide::get_castling_positions(side, Piece::King, castling_side);

        Move::Castle {
            from: from,
            to: to,
            side: castling_side,
        }
    }

    pub(crate) fn get_from_to(&self) -> (Square, Square) {
        match self {
            Move::Normal { from, to, .. } => return (*from, *to),
            &Move::Castle { from, to, .. } => return (from, to),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum CastlingSide {
    KingSide,
    QueenSide,
}

impl CastlingSide {
    pub(crate) const WHITE_KING_SIDE_EMPTY_MASK: u64 = Square::F1.bit() | Square::G1.bit();
    pub(crate) const WHITE_KING_SIDE_NOT_ATTACKED_MASK: u64 =
        CastlingSide::WHITE_KING_SIDE_EMPTY_MASK | Square::E1.bit();

    pub(crate) const WHITE_QUEEN_SIDE_EMPTY_MASK: u64 =
        Square::B1.bit() | Square::C1.bit() | Square::D1.bit();
    pub(crate) const WHITE_QUEEN_SIDE_NOT_ATTACKED_MASK: u64 =
        Square::C1.bit() | Square::D1.bit() | Square::E1.bit();

    pub(crate) const BLACK_KING_SIDE_EMPTY_MASK: u64 = Square::F8.bit() | Square::G8.bit();
    pub(crate) const BLACK_KING_SIDE_NOT_ATTACKED_MASK: u64 =
        CastlingSide::BLACK_KING_SIDE_EMPTY_MASK | Square::E8.bit();

    pub(crate) const BLACK_QUEEN_SIDE_EMPTY_MASK: u64 =
        Square::B8.bit() | Square::C8.bit() | Square::D8.bit();
    pub(crate) const BLACK_QUEEN_SIDE_NOT_ATTACKED_MASK: u64 =
        Square::C8.bit() | Square::D8.bit() | Square::E8.bit();

    pub(crate) fn get_castling_positions(
        side: Side,
        piece: Piece,
        castling_side: CastlingSide,
    ) -> (Square, Square) {
        if !&[Piece::King, Piece::Rook].contains(&piece) {
            panic!("Wrong piece type");
        }

        if side == Side::White {
            match (piece, castling_side) {
                (Piece::King, CastlingSide::KingSide) => return (Square::E1, Square::G1),
                (Piece::King, CastlingSide::QueenSide) => return (Square::E1, Square::C1),
                (Piece::Rook, CastlingSide::KingSide) => return (Square::H1, Square::F1),
                (Piece::Rook, CastlingSide::QueenSide) => return (Square::A1, Square::D1),
                _ => unreachable!(),
            }
        } else {
            match (piece, castling_side) {
                (Piece::King, CastlingSide::KingSide) => return (Square::E8, Square::G8),
                (Piece::King, CastlingSide::QueenSide) => return (Square::E8, Square::C8),
                (Piece::Rook, CastlingSide::KingSide) => return (Square::H8, Square::F8),
                (Piece::Rook, CastlingSide::QueenSide) => return (Square::A8, Square::D8),
                _ => unreachable!(),
            }
        }
    }
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
    pub(crate) struct MoveFlags: u8 {
        const NONE        = 0;
        const EN_PASSANT  = 1 << 0;
        const DOUBLE_PUSH = 1 << 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_index_tests() {
        assert_eq!(Square::A1.index(), 0);
        assert_eq!(Square::A4.index(), 24);
        assert_eq!(Square::H8.index(), 63);
    }

    #[test]
    fn square_bit_tests() {
        assert_eq!(Square::A1.bit(), 1);
        assert_eq!(Square::H1.bit(), 128);
        assert_eq!(Square::H8.bit(), 1u64 << 63);
    }

    #[test]
    fn square_rank_tests() {
        assert_eq!(Square::A1.rank().index(), 0);
        assert_eq!(Square::G4.rank().index(), 3);
        assert_eq!(Square::B8.rank().index(), 7);
    }

    #[test]
    fn square_file_tests() {
        assert_eq!(Square::A1.file().index(), 0);
        assert_eq!(Square::C3.file().index(), 2);
        assert_eq!(Square::F4.file().index(), 5);
    }

    #[test]
    fn square_to_string_tests() {
        assert_eq!(Square::A1.to_string(), "a1");
        assert_eq!(Square::E4.to_string(), "e4");
        assert_eq!(Square::H8.to_string(), "h8");
    }

    #[test]
    fn square_try_from_tests() {
        assert_eq!(Square::try_from(0).unwrap(), Square::A1);
        assert_eq!(Square::try_from(63).unwrap(), Square::H8);
    }

    #[test]
    #[ignore]
    fn test_move_size() {
        println!("Move size: {}", std::mem::size_of::<Move>());
    }
}
