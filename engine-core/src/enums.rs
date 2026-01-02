use std::fmt;

use crate::chess_consts;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Side {
    White,
    Black,
}

impl Side {
    #[inline]
    pub const fn index(self) -> u8 {
        self as u8
    }

    #[inline]
    pub const fn opposite(self) -> Side {
        match self {
            Self::White => Side::Black,
            Self::Black => Side::White,
        }
    }

    #[inline]
    pub const unsafe fn from_u8_unchecked(v: u8) -> Side {
        unsafe { std::mem::transmute(v) }
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
#[rustfmt::skip]
pub enum Square {
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
    pub const fn index(self) -> u8 {
        self as u8
    }

    #[inline]
    pub const fn bit(self) -> u64 {
        1u64 << (self as u64)
    }

    #[inline]
    pub const fn rank(self) -> u8 {
        self.index() / 8
    }

    #[inline]
    pub const fn file(self) -> u8 {
        self.index() % 8
    }

    #[inline]
    pub const unsafe fn from_u8_unchecked(v: u8) -> Square {
        unsafe { std::mem::transmute(v) }
    }

    pub(crate) fn all() -> impl Iterator<Item = Square> {
        Square::range(Square::A1, Square::H8)
    }

    pub(crate) fn range(from: Square, to: Square) -> impl Iterator<Item = Square> {
        (from.index()..=to.index()).map(|v| unsafe { Square::from_u8_unchecked(v) })
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

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let idx = *self as u8;
        let file = (b'a' + (idx % 8)) as char;
        let rank = (idx / 8) + 1;
        write!(f, "{file}{rank}")
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
#[rustfmt::skip]
pub enum File { A=0, B=1, C=2, D=3, E=4, F=5, G=6, H=7 }

impl File {
    pub const fn index(self) -> u8 {
        self as u8
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
#[derive(Copy, Clone)]
#[rustfmt::skip]
pub enum Rank { R1=0, R2=1, R3=2, R4=3, R5=4, R6=5, R7=6, R8=7 }

impl Rank {
    pub const fn index(self) -> u8 {
        self as u8
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
#[derive(Clone, Copy)]
#[rustfmt::skip]
pub enum Piece {Pawn, Knight, Bishop, Rook, Queen, King}

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
        assert_eq!(Square::A1.rank(), 0);
        assert_eq!(Square::G4.rank(), 3);
        assert_eq!(Square::B8.rank(), 7);
    }

    #[test]
    fn square_file_tests() {
        assert_eq!(Square::A1.file(), 0);
        assert_eq!(Square::C3.file(), 2);
        assert_eq!(Square::F4.file(), 5);
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
}
