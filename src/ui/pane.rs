use crate::tty::{Tty, TtyError};


#[allow(dead_code)]
pub struct Pane<'t> {
    tty: &'t Tty,
    rect: Rect,
    cursor_pos: Pos
}

impl <'t> Pane <'t>{
    pub fn new <'a:'t>(tty: &'a mut Tty, rect: Rect) -> Self {
        let mut pane: Self = tty.try_into().unwrap();
        pane.rect = rect;
        pane
    }

    pub fn test (&mut self) {

    }
}

impl <'a:'t, 't> TryFrom<&'a mut Tty> for Pane <'t> {
    type Error = TtyError;
    fn try_from(value: &'a mut Tty) -> Result<Self, Self::Error>  {
        let size = value.size().unwrap();
        let rect = Rect {
            pos: Pos::ZERO,
            width: size.ws_col as usize,
            height: size.ws_row as usize,
        };
        Ok(Self { tty: value, rect, cursor_pos: Pos::ZERO })
    }
}

pub struct Rect {
    pub pos: Pos,
    pub width: usize,
    pub height: usize,
}

/// Represents position of upper-left corner of `Pane`
/// zero-indexed
pub struct Pos {
    pub row: usize,
    pub col: usize,
}

impl Pos {
    pub const ZERO: Self = Self {row: 0, col: 0};

    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}
