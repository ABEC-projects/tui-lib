use crate::tty::{Tty, self};
use crate::utils::{ArenaAlloc, ArenaHandle};

pub struct Tui {
    tty: Tty,
    anchors: ArenaAlloc<(Anchor, RectHandle)>,
}

impl Tui {

    pub fn new() -> tty::Result<Self> {
        let tty = Tty::new()?;
        Ok( Self {
            tty,
            anchors: ArenaAlloc::new(),
        })
    }

    pub fn add_anchor(&mut self, new: Anchor, relative_to: RectHandle) -> AnchorHandle {
        let handle = self.anchors.insert((new, relative_to));
        AnchorHandle::new(handle)
    }

}


pub enum Anchor {
    Absolute (isize, isize),
    Relative (f32, f32),
}

pub struct AnchorHandle {
    handle: ArenaHandle<(Anchor, RectHandle)>,
}

impl AnchorHandle {
    fn new(handle: ArenaHandle<(Anchor, RectHandle)>) -> Self {
        Self { handle }
    }
}

pub struct Cords {
    col: usize,
    row: usize,
}

impl Cords {
    pub fn new(col: usize, row: usize) -> Self {
        Self {col, row}
    }
}

impl Cords {
    pub const ZERO: Self = Self {col: 0, row: 0};
}

pub struct Rect {
    pub upper_left: Cords,
    pub down_right: Cords,
}

pub struct RectHandle {
    handle: ArenaHandle<Rect>
}

impl RectHandle {
    fn new(handle: ArenaHandle<Rect>) -> Self {
        Self { handle }
    }
}
