use std::sync::{Arc, Mutex};

use super::tty;


#[allow(dead_code)]
pub struct Pane {
    tty: Arc<Mutex<tty::Tty>>,
    size: Size,
    position: Pos,
}

impl Pane {
}

impl From<tty::Tty> for Pane {
    fn from(mut value: tty::Tty) -> Self {
        let size = Size::from(value.size().unwrap());
        let position = Pos {row: 0, col: 0};
        let tty = Arc::new(Mutex::new(value));
        Self { tty, size, position }
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

#[allow(dead_code)]
pub struct Pos {
    row: usize,
    col: usize,
}
