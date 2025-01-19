use nixtui_core::{
    input::{constants, InputParser, KeyCode, KeyEvent},
    tty::Tty,
};
use std::io::{Read, Write};

fn main() {
    let items = ["foo", "bar", "baz", "cow"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut selector = Selector::new(items);
    if let Some(selected) = selector.run() {
        println!("{}", selected);
    }
}

struct Selector {
    tty: Tty,
    parser: InputParser,
    items: Vec<String>,
    cursor_pos: usize,
}

impl Selector {
    fn new(items: Vec<String>) -> Self {
        assert_ne!(items.len(), 0);
        let tty = Tty::new().unwrap();
        Self {
            parser: tty.make_parser(),
            tty,
            items,
            cursor_pos: 0,
        }
    }

    fn run(&mut self) -> Option<&str> {
        self.tty.raw_mode().unwrap();
        self.tty.enter_ca_mode().unwrap();
        self.tty.cursor_invisible().unwrap();

        let mut cancelled = false;

        'loop_: loop {
            for (i, s) in self.items.iter().enumerate() {
                self.tty.move_cursor(i, 0).unwrap();
                if i == self.cursor_pos {
                    self.tty.reverse().unwrap();
                }
                self.tty.write_all(s.as_bytes()).unwrap();
                if i == self.cursor_pos {
                    self.tty.exit_attribute_modes().unwrap();
                }
            }
            self.tty.move_cursor(self.cursor_pos, 0).unwrap();
            let mut buf = [0; 16];
            let count = self.tty.read(buf.as_mut()).unwrap();
            let parsed = self.parser.parse(&buf[0..count]);
            for byte in parsed.iter() {
                match byte {
                    KeyEvent { key_code, .. } if key_code.0 == b'\r' as u32 => break 'loop_,
                    KeyEvent { key_code, .. }
                        if key_code.0 == b'\x1B' as u32 || key_code.0 == b'q' as u32 =>
                    {
                        cancelled = true;
                        break 'loop_;
                    }
                    KeyEvent {
                        key_code: KeyCode(constants::UP),
                        ..
                    } => self.cursor_pos = self.cursor_pos.saturating_sub(1),
                    KeyEvent {
                        key_code: KeyCode(constants::DOWN),
                        ..
                    } => {
                        if self.cursor_pos < self.items.len() - 1 {
                            self.cursor_pos += 1
                        }
                    }
                    _ => {}
                }
            }
        }
        self.tty.cursor_normal_visibility().unwrap();
        self.tty.exit_ca_mode().unwrap();
        self.tty.write_orig_termios().unwrap();
        if !cancelled {
            Some(&self.items[self.cursor_pos])
        } else {
            None
        }
    }
}
