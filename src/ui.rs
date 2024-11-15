use nixtui_allocator::{ArenaAlloc, ArenaHandle};

type AnchorArenaHandle = ArenaHandle<(Anchor, Option<RectHandle>)>;


pub struct TuiAnchors  {
    anchors: ArenaAlloc<(Anchor, Option<RectHandle>)>,
    size: Rect,
}

impl TuiAnchors {

    pub fn new(size: Rect) -> Self {
        let anchors = ArenaAlloc::new();
        Self {
            anchors,
            size,
        }
    }


    pub fn add_anchor_in(&mut self, anchor: Anchor, relative_to: &RectHandle) -> AnchorHandle {
        let handle = self.anchors.insert((anchor, Some(relative_to.clone())));
        AnchorHandle::new(handle)
    }

    pub fn add_anchor(&mut self, anchor: Anchor,) -> AnchorHandle {
        self.anchors.insert((anchor, None)).into()
    }

    pub fn add_rect(&mut self, upper_left: &AnchorHandle, down_right: &AnchorHandle) -> RectHandle {
        RectHandle::new(&upper_left.0, &down_right.0)
    }
    
    pub fn get_cords_of_anchor(&self, handle: &AnchorHandle) -> Cords {
        self.raw_get_cords_of_anchor(&handle.0)
    }

    fn raw_get_cords_of_anchor(&self, handle: &AnchorArenaHandle) -> Cords {
        let (anchor, rect) = self.anchors.get(handle).unwrap();
        let rect = match rect {
            Some(rh) => {
                let upper_left = self.raw_get_cords_of_anchor(&rh.upper_left.clone());
                let down_right = self.raw_get_cords_of_anchor(&rh.down_right.clone());
                Rect::new(upper_left, down_right)
            },
            None => self.size.clone(),
        };
        let col = match anchor.col_offset {
            Offset::Absolute(i) if !anchor.from_right => rect.upper_left.col.saturating_add_signed(i)
                .clamp(0, self.size.down_right.col),
                
            Offset::Absolute(i) if anchor.from_right => rect.down_right.col.saturating_add_signed(-i)
                .clamp(0, self.size.down_right.col),

            Offset::Relative(f) if !anchor.from_down =>
                (rect.upper_left.col as f32 + (rect.down_right.col.saturating_sub(rect.upper_left.col)) as f32 * f)
                .clamp(0., self.size.down_right.col as f32) as usize,

            Offset::Relative(f) if anchor.from_down =>
                (rect.upper_left.col as f32 + (rect.down_right.col.saturating_sub(rect.upper_left.col)) as f32 * (1.-f))
                .clamp(0., self.size.down_right.col as f32) as usize,

            _ => unreachable!()
        };
        let row = match anchor.row_offset {
            Offset::Absolute(i) if !anchor.from_right => rect.upper_left.row.saturating_add_signed(i)
                .clamp(0, self.size.down_right.row),
                
            Offset::Absolute(i) if anchor.from_right => rect.down_right.row.saturating_add_signed(-i)
                .clamp(0, self.size.down_right.row),

            Offset::Relative(f) if !anchor.from_down =>
                (rect.upper_left.row as f32 + (rect.down_right.row.saturating_sub(rect.upper_left.row)) as f32 * f)
                .clamp(0., self.size.down_right.row as f32) as usize,

            Offset::Relative(f) if anchor.from_down =>
                (rect.upper_left.row as f32 + (rect.down_right.row.saturating_sub(rect.upper_left.row)) as f32 * (1.-f))
                .clamp(0., self.size.down_right.row as f32) as usize,

            _ => unreachable!()
        };
        Cords {row, col}
    }

    pub fn update_size(&mut self, size: Rect) {
        self.size = size;
    }
}

#[derive(Debug, Clone)]
pub struct Anchor {
    col_offset: Offset,
    from_right: bool,
    row_offset: Offset,
    from_down: bool,
}

impl Anchor {
    pub fn new(col_offset: Offset, from_right: bool, row_offset: Offset, from_down: bool) -> Self {
        Self { col_offset, from_right, row_offset, from_down }
    }

    pub fn new_abs_from_upper_left (col: isize, row: isize) -> Self {
        Self { col_offset: Offset::Absolute(col), from_right: false, row_offset: Offset::Absolute(row), from_down: false }
    }

    pub fn new_abs_from_down_right (col: isize, row: isize) -> Self {
        Self { col_offset: Offset::Absolute(col), from_right: true, row_offset: Offset::Absolute(row), from_down: true }
    }

    pub fn new_rel_from_down_right (col: f32, row: f32) -> Self {
        Self { col_offset: Offset::Relative(col), from_right: true, row_offset: Offset::Relative(row), from_down: true }
    }

    pub fn new_rel_from_upper_left (col: f32, row: f32) -> Self {
        Self { col_offset: Offset::Relative(col), from_right: false, row_offset: Offset::Relative(row), from_down: false }
    }
}

#[derive(Debug, Clone)]
pub enum Offset {
    Absolute(isize),
    Relative(f32),
}

pub struct AnchorHandle (AnchorArenaHandle,);

impl AnchorHandle {
    fn new(handle: AnchorArenaHandle) -> Self {
        Self (handle)
    }
}

impl From<AnchorArenaHandle> for AnchorHandle {
    fn from(val: AnchorArenaHandle) -> Self {
        AnchorHandle(val)
    }
}

impl From<AnchorHandle> for AnchorArenaHandle {
    fn from(value: AnchorHandle) -> Self {
        value.0
    }
}

#[derive(Clone, Debug)]
pub struct Cords {
    pub col: usize,
    pub row: usize,
}

impl Cords {
    pub fn new(col: usize, row: usize) -> Self {
        Self {col, row}
    }
}

impl Cords {
    pub const ZERO: Self = Self {col: 0, row: 0};
}

impl From<(usize, usize)> for Cords {
    fn from(value: (usize, usize)) -> Self {
        Self { col: value.0, row: value.1 }
    }
}


#[derive(Clone, Debug)]
pub struct Rect {
    pub upper_left: Cords,
    pub down_right: Cords,
}

impl Rect {
    pub fn new(upper_left: Cords, down_right: Cords) -> Self {
        Self { upper_left, down_right }
    }
}

impl From<nix::libc::winsize> for Rect {
    fn from(value: nix::libc::winsize) -> Self {
        let (col, row): (usize, usize) = (value.ws_col.into(), value.ws_row.into());
        Self { upper_left: Cords::ZERO, down_right: Cords::new(col-1, row-1)}
    }
}

#[derive(Debug, Clone)]
pub struct RectHandle {
    upper_left: AnchorArenaHandle,
    down_right: AnchorArenaHandle,
}

impl RectHandle {
    fn new(upper_left: &AnchorArenaHandle, down_right: &AnchorArenaHandle) -> Self {
        Self { upper_left: upper_left.clone(), down_right: down_right.clone() }
    }
}
