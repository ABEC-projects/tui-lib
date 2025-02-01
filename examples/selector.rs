use nix::sys::termios::Termios;
use nixtui_core::{
    input::{constants, InputParser, KeyCode, KeyEvent},
    tty::{TerminfoWrapper, UnixTerminal},
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
    tty: std::fs::File,
    terminfo: TerminfoWrapper,
    parser: InputParser,
    items: Vec<String>,
    cursor_pos: usize,
    orig_termios: Termios,
}

impl Selector {
    fn new(items: Vec<String>) -> Self {
        assert_ne!(items.len(), 0);
        let mut tty = std::fs::File::options()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .unwrap();
        let terminfo = TerminfoWrapper::from_env().unwrap();
        Self {
            parser: InputParser::from_terminfo(&terminfo.db),
            orig_termios: tty.get_termios().unwrap(),
            terminfo,
            tty,
            items,
            cursor_pos: 0,
        }
    }

    fn run(&mut self) -> Option<&str> {
        self.tty.raw_mode().unwrap();
        self.terminfo.enter_ca_mode().unwrap();
        self.terminfo.cursor_invisible().unwrap();
        self.terminfo.flush_to(&mut self.tty).unwrap();

        let mut cancelled = false;

        'loop_: loop {
            for (i, s) in self.items.iter().enumerate() {
                self.terminfo.move_cursor(i, 0).unwrap();
                if i == self.cursor_pos {
                    self.terminfo.enter_reverse_mode().unwrap();
                }
                self.terminfo.write_all(s.as_bytes()).unwrap();
                if i == self.cursor_pos {
                    self.terminfo.exit_attribute_mode().unwrap();
                }
            }
            self.terminfo.move_cursor(self.cursor_pos, 0).unwrap();
            self.terminfo.flush_to(&mut self.tty).unwrap();
            let mut buf = [0; 4095];
            let count = self.tty.read(buf.as_mut()).unwrap();
            let parsed = self.parser.parse(&buf[0..count]);
            for byte in parsed.iter() {
                match byte {
                    KeyEvent { key_code, .. }
                        if key_code.0 == b'\r' as u32 || key_code.0 == b'e' as u32 =>
                    {
                        break 'loop_
                    }
                    KeyEvent { key_code, .. }
                        if key_code.0 == b'\x1B' as u32 || key_code.0 == b'q' as u32 =>
                    {
                        cancelled = true;
                        break 'loop_;
                    }
                    KeyEvent {
                        key_code: KeyCode(constants::UP) | KeyCode(0x77),
                        ..
                    } => self.cursor_pos = self.cursor_pos.saturating_sub(1),
                    KeyEvent {
                        key_code: KeyCode(constants::DOWN) | KeyCode(0x73),
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
        self.terminfo.cursor_normal().unwrap();
        self.terminfo.exit_ca_mode().unwrap();
        self.terminfo.flush_to(&mut self.tty).unwrap();
        self.tty
            .set_termios(&self.orig_termios, nix::sys::termios::SetArg::TCSADRAIN)
            .unwrap();
        if !cancelled {
            Some(&self.items[self.cursor_pos])
        } else {
            None
        }
    }
}

impl Drop for Selector {
    fn drop(&mut self) {
        let _ = self
            .tty
            .set_termios(&self.orig_termios, nix::sys::termios::SetArg::TCSADRAIN);
        let _ = self.terminfo.exit_ca_mode();
        let _ = self.terminfo.exit_attribute_mode();
        let _ = self.terminfo.flush_to(&mut self.tty);
    }
}
