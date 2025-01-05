#![allow(dead_code)]

use terminfo::Database;

macro_rules! call_multiple {
    ($f:ident, [$($arg:expr),+$(,)?]) => {
        $($f($arg);)+ 
    };
    ($f:expr, [$($arg:expr),+$(,)?]) => {
        $(($f)($arg);)+ 
    };
    ($f:ident, $count:expr) => {
        for _ in 0..$count {
            $f();
        }
    };
    ($f:expr, $count:expr) => {
        for _ in 0..$count {
            ($f)();
        }
    };
}

macro_rules! push_from_db {
    ($db:ident, $to:expr, [$(($cap:path, $val:literal)),+$(,)?]) => {
        $(match $db.get::<$cap>() {
            Some(v) => {
                if let Some(slice) = &v.as_ref().get(2..) {
                    match CSICommand::parse(slice) {
                        Some(command) => {
                            $to.push(command.0, $val)
                        },
                        None => {}
                    }
                }
            },
            None => {},
        };
        )+
    };
}

#[derive(Default, Debug)]
pub struct InputParser {
    mappings: CSIList,
}

impl InputParser {

    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_env() -> Result<Self, terminfo::Error> {
        Ok(Self::from_terminfo(&Database::from_env()?))
    }

    pub fn from_terminfo(db: &Database) -> Self {
        let mut ret = Self::new();
        ret.push_from_terminfo(db);
        ret
    }

    pub fn push_from_terminfo(&mut self, db: &Database) {
        use terminfo::capability as cap;
        call_multiple!(|val: (&[u8], u32)| {
            if let Some(command) = CSICommand::parse(val.0)
            { self.mappings.push(command.0, val.1) }
        }, [
            (b"\x1B27u", 57344),
            (b"\x1B13u", 57345),
            (b"\x1B9u", 57346),
        ]);
        push_from_db!(db, self.mappings, [
            (cap::Tab, 57347),
            (cap::KeyBackspace, 57348),
            (cap::KeyIc, 57349),
            (cap::KeyDc, 57350),
            (cap::KeyLeft, 57351),
            (cap::KeyRight, 57352),
            (cap::KeyUp, 57353),
            (cap::KeyDown, 57354),
            (cap::KeyPPage, 57355), // PageUp
            (cap::KeyNPage, 57356), // PageDown
            (cap::KeyHome, 57357),
            (cap::CursorHome, 57357),
            (cap::KeyEnd, 57358),
        ]);
        call_multiple!(|val: (&[u8], u32)| {
            if let Some(command) = CSICommand::parse(val.0)
            { self.mappings.push(command.0, val.1) }
        }, [
            (b"\x1B57358u", 57358),
            (b"\x1B57359u", 57359),
            (b"\x1B57360u", 57360),
            (b"\x1B57361u", 57361),
            (b"\x1B57362u", 57362),
            (b"\x1B57363u", 57363),
        ]);
        push_from_db!(db, self.mappings, [
            (cap::KeyF1, 57364),
            (cap::KeyF2, 57365),
            (cap::KeyF3, 57366),
            (cap::KeyF4, 57367),
            (cap::KeyF5, 57368),
            (cap::KeyF6, 57369),
            (cap::KeyF7, 57370),
            (cap::KeyF8, 57371),
            (cap::KeyF9, 57372),
            (cap::KeyF10, 57373),
            (cap::KeyF11, 57374),
            (cap::KeyF12, 57375),
            (cap::KeyF13, 57376),
            (cap::KeyF14, 57377),
            (cap::KeyF15, 57378),
            (cap::KeyF16, 57379),
            (cap::KeyF17, 57380),
            (cap::KeyF18, 57381),
            (cap::KeyF19, 57382),
            (cap::KeyF20, 57383),
            (cap::KeyF21, 57384),
            (cap::KeyF22, 57385),
            (cap::KeyF23, 57386),
            (cap::KeyF24, 57387),
            (cap::KeyF25, 57388),
            (cap::KeyF26, 57389),
            (cap::KeyF27, 57390),
            (cap::KeyF28, 57391),
            (cap::KeyF29, 57392),
            (cap::KeyF30, 57393),
            (cap::KeyF31, 57394),
            (cap::KeyF32, 57395),
            (cap::KeyF33, 57396),
            (cap::KeyF34, 57397),
            (cap::KeyF35, 57398),
        ]);

    }

    pub fn parse(&self, input: &[u8]) -> Vec<KeyEvent> {
        let mut events = Vec::new();
        let mut iter = input.iter().enumerate();
        while let Some((i, byte)) = iter.next() {
            let byte = *byte;
            events.push(match byte {
                0x1B => {
                    match input.get(i+1) {
                        Some(next) => {
                            let i = i + 1;
                            let next = *next;
                            match next {
                                b'[' | b'O' => {
                                    if let Some(slice) = input.get((i+1)..) {
                                        if let Some((command, len)) = CSICommand::parse(slice) {
                                            iter.nth(len);
                                            if let Some(code) = self.mappings.match_csi(&command) {
                                                let mods = 'm: {match command.get_final() {
                                                    b'A'..=b'Z' | b'~' => {
                                                        if let Some(bytes) = command.get_parameter().split(|b|*b==b';').nth(1) {
                                                            let mut num = 0;
                                                            if bytes.len() > 3 {
                                                                break 'm Modifiers::NONE;
                                                            }
                                                            for (i, dig) in bytes.iter().rev().enumerate() {
                                                                if !(48..58).contains(dig) {
                                                                    break 'm Modifiers::NONE;
                                                                }
                                                                num += (dig-48)*10_u8.pow(i as u32)
                                                            }
                                                            Modifiers::new(num-1)
                                                        } else {
                                                            Modifiers::NONE
                                                        }
                                                    },
                                                    _ => Modifiers::NONE,
                                                }};
                                                KeyEvent {
                                                    key_code: code.into(),
                                                    mods,
                                                    ..Default::default()
                                                }
                                            } else {
                                                continue;
                                            }
                                        } else if next == b'[' {
                                            iter.next();
                                            KeyEvent {
                                                key_code: b'['.into(),
                                                mods: Modifiers::ALT,
                                                ..Default::default()
                                            }
                                        } else {
                                            iter.next();
                                            continue;
                                        }
                                    } else if next == b'[' {
                                        iter.next();
                                        KeyEvent {
                                            key_code: b'['.into(),
                                            mods: Modifiers::ALT,
                                            ..Default::default()
                                        }
                                    } else {
                                        break;
                                    }
                                },
                                0x20..=0x7E => {
                                    iter.next();
                                    KeyEvent {
                                        key_code: next.into(),
                                        mods: Modifiers::ALT,
                                        ..Default::default()
                                    }
                                },
                                _ => {
                                    KeyEvent {
                                        key_code: 0x1B_u8.into(),
                                        ..Default::default()
                                    }
                                }
                            }
                        },
                        None => {
                            KeyEvent {
                                key_code: 0x1B_u8.into(),
                                ..Default::default()
                            }
                        }
                    }
                },
                // ASCII
                0..=0x7F => {
                    KeyEvent {
                        key_code: byte.into(),
                        ..Default::default()
                    }
                },
                // Continuation byte
                0x80..=0xBF => {continue;},
                // First byte of 2-byte encoding
                0xC2..=0xDF => {
                    let byte2 = (byte as u32 & !(0b111 << 5)) << 6;
                    let byte1 = match iter.next().map(|x|x.1) {
                        Some(b) => *b,
                        None => continue,
                    } as u32 & !(0b11 << 6);
                    KeyEvent {
                        key_code: (byte2 | byte1).into(),
                        ..Default::default()
                    }
                },
                // First byte of 3-byte encoding
                0xE0..=0xEF => {
                    let byte1 = (byte as u32 & !(0b1111 << 4)) << 12;
                    let byte2 = (match iter.next().map(|x|x.1) {
                        Some(b) => *b,
                        None => continue,
                    } as u32 & !(0b11 << 6)) << 6; 
                    let byte3 = (match iter.next().map(|x|x.1) {
                        Some(b) => *b,
                        None => continue,
                    } as u32 & !(0b11 << 6)); 

                    KeyEvent {
                        key_code: (byte3 | byte2 | byte1).into(),
                        ..Default::default()
                    }
                },
                // First byte of 4-byte encoding
                0xF0..=0xF4 => {
                    let byte1 = (byte as u32 & !(0b11111 << 3)) << 20;
                    let byte2 = (match iter.next().map(|x|x.1) {
                        Some(b) => *b,
                        None => continue,
                    } as u32 & !(0b11 << 6)) << 12; 
                    let byte3 = (match iter.next().map(|x|x.1) {
                        Some(b) => *b,
                        None => continue,
                    } as u32 & !(0b11 << 6)) << 6; 
                    let byte4 = (match iter.next().map(|x|x.1) {
                        Some(b) => *b,
                        None => continue,
                    } as u32 & !(0b11 << 6)); 
                    KeyEvent {
                        key_code: KeyCode(byte1 | byte2 | byte3 | byte4),
                        ..Default::default()
                    }
                }
                // Unused in UTF-8
                0xC0..=0xC1 | 0xF5..=0xFF => {continue;},
            });
        }
        events
    }
}

#[derive(Default, Debug)]
struct CSIList {
    data: Vec<(CSICommand, u32)>
}

impl CSIList {

    fn new() -> Self {
        Self {
            data: Vec::new()
        }
    }
    
    fn push(&mut self, csi: CSICommand, codepoint: u32) {
        self.data.push((csi, codepoint));
    }
    
    fn find_by_codepoint(&self, codepoint: u32) -> Option<&CSICommand> {
        self.data.iter().find(|x|x.1 == codepoint).map(|x|&x.0)
    }

    fn match_csi(&self, csi: &CSICommand) -> Option<u32> {
        self.data.iter().find(|item| {
            match csi.get_final() {
                b'A'..=b'Z' => {
                    csi.get_final() == item.0.get_final()
                },
                b'~' => {
                    if item.0.get_final() == b'~' {
                        match csi.get_parameter().split(|x|*x==b';').next() {
                            Some(x) => x == item.0.get_parameter(),
                            None => false,
                        }
                    } else {false}
                },
                _ => false,
            }
        }).map(|x|x.1)
    }

}



#[derive(Clone, PartialEq, Eq, Debug)]
struct CSICommand {
    parameter_bytes: Vec<u8>,
    intermediate_bytes: Vec<u8>,
    final_byte: u8
}

impl CSICommand {
    fn get_parameter(&self) -> &[u8] {
        &self.parameter_bytes
    }
    fn get_intermediate(&self) -> &[u8] {
        &self.intermediate_bytes
    }
    fn get_final(&self) -> u8 {
        self.final_byte
    }

    fn parse(bytes: &[u8]) -> Option<(Self, usize)> {
        let mut skipped = false;
        let bytes = if bytes.get(0..2) == Some(b"\x1B[") {
            skipped = true;
            match bytes.get(2..) {
                Some(v) => v,
                None => return None,
            }
        }else {
            bytes
        };

        let mut interm = false;
        let mut param_end = 0;
        let mut inter_end = 0;
        let mut final_byte = 0;

        for byte in bytes {
            if !interm  {
                if (0x20..=0x2F).contains(byte) {
                    interm = true;
                    inter_end = param_end + 1;
                    continue;
                }
                if (0x40..=0x7E).contains(byte) {
                    inter_end = param_end;
                    final_byte = *byte;
                    break;
                }
                if !(0x30..=0x3F).contains(byte){
                    return None;
                }
                param_end += 1;
            }
            else {
                if (0x40..=0x7E).contains(byte) {
                    final_byte = *byte;
                    break;
                }
                if !(0x20..=0x2F).contains(byte) {
                    return None;
                }
                inter_end += 1;
            }
        }

        if final_byte == 0 {
            return None;
        }
        Some((
                Self {
                    parameter_bytes: bytes[0..param_end].to_vec(),
                    intermediate_bytes: bytes[param_end..inter_end].to_vec(),
                    final_byte,
                },
                inter_end + 1 + if skipped {2} else {0}
        )
        )
    }


}


#[derive(Default, Debug)]
pub struct KeyEvent {
    key_code: KeyCode,
    mods: Modifiers,
    event_type: EventType,
}

/// Used to represent any key as either 
/// standart unicode codepoint or codepoint from 
/// Unicode Private Use Area for most functional keys
#[derive(Default, Debug, PartialEq, Eq)]
struct KeyCode (u32);

impl From<u32> for KeyCode {
    fn from(val: u32) -> Self {
        KeyCode(val)
    }
}

impl From<u8> for KeyCode {
    fn from(value: u8) -> Self {
        Self(value as u32)
    }
}

enum FunctionalKey {
    Escape,
    Enter,
    Tab,
    Backspace,
    Insert,
    Delete,
    Left, 
    Right,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    CapsLock,
    ScrollLock,
    NumLock,
    PrintScreen,
    Pause,
    Menu,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    F25,
    F26,
    F27,
    F28,
    F29,
    F30,
    F31,
    F32,
    F33,
    F34,
    F35,
    KP1,
    KP2,
    KP3,
    KP4,
    KP5,
    KP6,
    KP7,
    KP8,
    KP9,
    KPDecimal,
    KPDivide,
    KPSubtract,
    KPAdd,
    KPEnter,
    KPEqual,
    KPSeparator,
    KPLeft,
    KPRight,
    KPUp,
    KPDown,
    KPPageUp,
    KPPageDown,
    KPInsert,
    KPDelete,
    KPHome,
    KPEnd,
    KPBegin,
    MediaPlay,
    MediaPause,
    MediaPlayPause,
    MediaReverse,
    MediaStop,
    MediaFastForward,
    MediaRewind,
    MediaTrackNext,
    MediaTrackPrevious,
    MediaRecord,
    LowerVolume,
    RaiseVolume,
    MuteVolume,
    LeftShift,
    LeftControl,
    LeftAlt,
    LeftSuper,
    LeftHypre,
    LeftMeta,
    RightShift,
    RightControl,
    RightAlt,
    RightSuper,
    RightHypre,
    RightMeta,
    IsoLevel3Shift,
    IsoLevel5Shift,
}

#[derive(Default, Debug)]
enum EventType {
    Press,
    #[default]
    Repeat,
    Release
}

//shift     0b1         (1)
//alt       0b10        (2)
//ctrl      0b100       (4)
//super     0b1000      (8)
//hyper     0b10000     (16)
//meta      0b100000    (32)
//caps_lock 0b1000000   (64)
//num_lock  0b10000000  (128)
#[derive(PartialEq, Eq, Hash, Clone, Copy, Default)]
struct Modifiers (u8);

impl Modifiers {
    pub const NONE: Self = Self(0);
    pub const SHIFT: Self = Self(1);
    pub const ALT: Self = Self(2);
    pub const CTRL: Self = Self(4);
    pub const SUPER: Self = Self(8);
    pub const HYPER: Self = Self(16);
    pub const META: Self = Self(32);
    pub const CAPS_LOCK: Self = Self(64);
    pub const NUM_LOCK: Self = Self(128);

    pub fn new(mods: u8) -> Self {
        Self(mods)
    }

    #[inline]
    pub fn shift_pressed(&self) -> bool {
        check_bit_at(self.0, 0)
    }
    #[inline]
    pub fn alt_pressed(&self) -> bool {
        check_bit_at(self.0, 1)
    }
    #[inline]
    pub fn ctrl_pressed(&self) -> bool {
        check_bit_at(self.0, 2)
    }
    #[inline]
    pub fn super_pressed(&self) -> bool {
        check_bit_at(self.0, 3)
    }
    #[inline]
    pub fn hyper_pressed(&self) -> bool {
        check_bit_at(self.0, 4)
    }
    #[inline]
    pub fn meta_pressed(&self) -> bool {
        check_bit_at(self.0, 5)
    }
    #[inline]
    pub fn caps_lock_pressed(&self) -> bool {
        check_bit_at(self.0, 6)
    }
    #[inline]
    pub fn num_lock_pressed(&self) -> bool {
        check_bit_at(self.0, 7)
    }


    #[inline]
    pub fn superset_of(&self, other: Self) -> bool {
        self.0 | other.0 == self.0
    }
    #[inline]
    pub fn subset_of(&self, other: Self) -> bool {
        self.0 | other.0 == other.0
    }
}

impl std::fmt::Debug for Modifiers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbs = f.debug_list();
        if self.shift_pressed() {
            dbs.entry(&"Shift");
        }
        if self.ctrl_pressed() {
            dbs.entry(&"Ctrl");
        }
        if self.alt_pressed() {
            dbs.entry(&"Alt");
        }
        if self.super_pressed() {
            dbs.entry(&"Super");
        }
        if self.hyper_pressed() {
            dbs.entry(&"Hyper");
        }
        if self.meta_pressed() {
            dbs.entry(&"Meta");
        }
        if self.caps_lock_pressed() {
            dbs.entry(&"CapsLock");
        }
        if self.num_lock_pressed() {
            dbs.entry(&"NumLock");
        }
        dbs.finish()
    }
}

fn check_bit_at(byte: u8, n: u8) -> bool {
    byte << (7-n) >> 7 == 1
}


impl std::ops::BitAnd for Modifiers {
    type Output = Self;
    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::BitOr for Modifiers {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitXor for Modifiers {
    type Output = Self;
    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl std::ops::BitOrAssign for Modifiers {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAndAssign for Modifiers {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl std::ops::BitXorAssign for Modifiers {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl std::ops::Not for Modifiers {
    type Output = Self;
    #[inline]
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn as_bin(num: u32) -> String {
        let mut s = String::from("0b");
        for i in (0..32).rev() {
            if num & (1_u32 << i) != 0 {
                s.push('1');
            } else {
                s.push('0');
            }
        }
        s
    }


    #[test]
    fn test_check_bit () {
        for i in 0..8_u8 {
            for j in 0..8_u8 {
                let mut should_pass = true;
                if j == i && i != 7 {should_pass = false;}
                let this = 2_u8.pow(i as u32) + if i == 7 && j == 7 {0} else { 2_u8.pow(j as u32) };
                assert_eq!(check_bit_at(this, i), should_pass, "This: {this}, i: {i}, j: {j}", );
            }
        }
    }

    #[test]
    fn check_sup_sub_set() {
        let a = Modifiers::CTRL | Modifiers::CAPS_LOCK;
        assert!(Modifiers::CTRL.subset_of(a));
        assert!(a.superset_of(Modifiers::CTRL));
        assert!(!Modifiers::ALT.subset_of(a));
        assert!(!Modifiers::ALT.superset_of(a));
    }

    #[test]
    fn test_parser() {
        let parser = InputParser::from_env().unwrap();
        // Cyrilic Б
        let parsed = parser.parse(b"\xD0\x91")[0].key_code.0;
        assert_eq!(parsed, 0x411, "\n {parsed}: {}", as_bin(parsed));
        // અ
        let parsed = parser.parse(b"\xE0\xAA\x85")[0].key_code.0;
        assert_eq!(parsed, 0xA85, "\n {parsed}: {}", as_bin(parsed));
        // 😭
        let parsed = parser.parse(b"\xF0\x9F\x98\xAD")[0].key_code.0;
        assert_eq!(parsed, 0x1F62D, "\n {parsed}: {}", as_bin(parsed));
    }

    #[test]
    fn test_call_multiple() {
        let mut num = 0;
        let mut cl = |x|{num+=x;};
        call_multiple!({||cl(1)}, 10);
        assert_eq!(num, 10);
        let mut num2 = 0;
        let mut cl = |x|{num2+=x;};
        call_multiple!(cl, [1, 2, 3, 4]);
        assert_eq!(num2, 10);
    }


    #[test]
    fn test_csi_parser() {
        let res = CSICommand::parse(b"\x1B[109;109###Hasd").unwrap();
        assert_eq!(res.0, CSICommand{
            parameter_bytes: b"109;109".to_vec(),
            intermediate_bytes: b"###".to_vec(),
            final_byte: b'H',
        });
        assert_eq!(res.1, 13);
        let res = CSICommand::parse(b"109;109###Hasd").unwrap();
        assert_eq!(res.0, CSICommand{
            parameter_bytes: b"109;109".to_vec(),
            intermediate_bytes: b"###".to_vec(),
            final_byte: b'H',
        });
        assert_eq!(res.1, 11);
        let res = CSICommand::parse(b"\x1B[B").unwrap().0;
        assert_eq!(res, CSICommand{
            parameter_bytes: b"".to_vec(),
            intermediate_bytes: b"".to_vec(),
            final_byte: b'B',
        });
        let res = CSICommand::parse(b"\x1B[###~").unwrap().0;
        assert_eq!(res, CSICommand{
            parameter_bytes: b"".to_vec(),
            intermediate_bytes: b"###".to_vec(),
            final_byte: b'~',
        });
    }

    #[test]
    fn test_csi_list() {
        let csi = CSICommand {
            parameter_bytes: b"2;5".to_vec(),
            intermediate_bytes: Vec::new(),
            final_byte: b'~',
        };
        let mut list = CSIList::new();
        list.push(CSICommand::parse(b"2~").unwrap().0, 57349);
        assert_eq!(list.match_csi(&csi), Some(57349));
    }

}
