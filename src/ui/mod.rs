use crate::tty::Tty;


#[allow(dead_code)]
pub struct Pane<'t> {
    tty: &'t Tty,
    size: Size,
    position: Pos,
}

impl <'t> Pane <'t>{
    pub fn new <'a:'t>(tty: &'a mut Tty, size: Size, pos: Pos) -> Self {
        let mut pane: Self = tty.into();
        pane.size = size;
        pane.position = pos;
        pane
    }
}

impl <'a:'t, 't> From<&'a mut Tty> for Pane <'t> {
    fn from(value: &'a mut Tty) -> Self {
        let size = Size::from(value.size().unwrap());
        let position = Pos {row: 0, col: 0};
        Self { tty: value, size, position }
    }
}

#[allow(dead_code)]
pub struct Size {
    width: usize,
    height: usize,
}

impl From<nix::libc::winsize> for Size {
    fn from(value: nix::libc::winsize) -> Self {
        Self {
            width: value.ws_col as usize,
            height: value.ws_row as usize,
        }
    }
}

/// Represents position of upper-left corner of `Pane`
/// zero-indexed
#[allow(dead_code)]
pub struct Pos {
    row: usize,
    col: usize,
}
