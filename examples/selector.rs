use nix::{libc::{VMIN, VTIME}, sys::termios::SetArg};
use nixtui::tty::Tty;
use std::io::{Read, Write};

fn main() {
    let items = ["foo", "bar", "baz", "cow"].iter().map(|s|s.to_string()).collect();
    let mut selector = Selector::new(items);
    let s = selector.run();
    println!("{}",s);
}

struct Selector {
    tty: Tty,
    items: Vec<String>,
    cursor_pos: usize,
}

impl Selector {
    fn new(items: Vec<String>) -> Self{
        assert_ne!(items.len(), 0);
        let tty = Tty::new().unwrap();
        Self { tty, items, cursor_pos: 0 }
    }

    fn run(&mut self) -> &str {
        self.tty.raw_mode().unwrap();
        let mut termios = self.tty.get_termios().unwrap().clone();
        termios.control_chars[VTIME] = 1;
        termios.control_chars[VMIN] = 0;
        self.tty.write_termios(termios, SetArg::TCSAFLUSH).unwrap();
        self.tty.cursor_invisible().unwrap();

        'loop_: loop {
            for (i, s) in self.items.iter().enumerate() {
                self.tty.move_cursor(i, 0).unwrap();
                if i == self.cursor_pos {self.tty.write_all(b"\x1B[7m").unwrap();}
                self.tty.write_all(s.as_bytes()).unwrap();
                if i == self.cursor_pos {self.tty.write_all(b"\x1B[0m").unwrap();}
            }
            self.tty.move_cursor(self.cursor_pos, 0).unwrap();
            let mut buf = [0;16];
            let count = self.tty.raw.read(buf.as_mut()).unwrap();
            for byte in &buf[0..count] {
                match byte {
                    b'\n' => break 'loop_,
                    b'\x1B' if count == 1 => break 'loop_,
                    b'\x1B' if count == 3 => {
                        match &buf[1..count] {
                            b"[A" => self.cursor_pos = self.cursor_pos.saturating_sub(1),
                            b"[B" => if self.cursor_pos < self.items.len()-1 {self.cursor_pos += 1},
                            _ =>(),
                        }; 
                        break
                    },
                    _ => (),
                }
            }
        }
        self.tty.show_cursor().unwrap();
        self.tty.recook().unwrap();
        &self.items[self.cursor_pos]
    }
}
