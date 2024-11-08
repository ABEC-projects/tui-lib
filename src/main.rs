use nix::{libc::{VMIN, VTIME}, sys::termios::SetArg};
use tui::tty::Tty;
use std::io::{Read, Write};

fn main() {
    // let mut tty = tty::Tty::new().unwrap();
    // tty.uncook().unwrap();
    // tty.move_cursor(0, 0).unwrap();
    // tty.write_all(b"foo").unwrap();
    // tty.move_cursor(1, 0).unwrap();
    // tty.write_all(b"bar").unwrap();
    // tty.move_cursor(2, 0).unwrap();
    // tty.write_all(b"baz").unwrap();
    // tty.move_cursor(0, 0).unwrap();
    // let mut buf = [0;1];
    // let mut line = 0_u8;
    // loop {
    //     let count = tty.raw.read(&mut buf).unwrap();
    //     if count == 0 {continue;}
    //     if buf[0] != b'\x1B' {
    //         if buf [0] == b'\n' {
    //             break;
    //         }
    //     }else {
    //         let mut escape_buf = vec![0; 8];   
    //         let mut termios = tty.get_termios().unwrap();
    //         termios.control_chars[VTIME] = 1;
    //         termios.control_chars[VMIN] = 0;
    //         tty.write_termios(termios, SetArg::TCSANOW).unwrap();
    //         let count = tty.raw.read(&mut escape_buf).unwrap();
    //         if count < 2 {break};
    //         match &escape_buf[0..=1] {
    //             b"[A" => line = line.saturating_sub(1),
    //             b"[B" => if line != 2 {line += 1},
    //             _ => ()
    //         }
    //         tty.move_cursor(line as usize, 0).unwrap();
    //     }
    // }
    // tty.recook().unwrap();
    // let s = match line {
    //     0 => "foo",
    //     1 => "bar",
    //     2 => "baz",
    //     _ => unreachable!(),
    // };
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
        self.tty.uncook().unwrap();
        let old_termios = self.tty.get_termios().unwrap();
        let mut termios = old_termios.clone();
        termios.control_chars[VTIME] = 1;
        termios.control_chars[VMIN] = 0;
        self.tty.write_termios(termios, SetArg::TCSAFLUSH).unwrap();
        self.tty.hide_cursor().unwrap();

        'outer: loop {
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
                    b'\n' => break 'outer,
                    b'\x1B' if count == 1 => break 'outer,
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

