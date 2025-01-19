use std::{
    io::{Read, Write},
    time::Duration,
};

use nixtui_core::tty::Tty;

fn main() {
    test_tty_expand();
}

fn get_cap() {
    use terminfo::capability as cap;
    let db = terminfo::Database::from_env().unwrap();
    let c = db.get::<cap::ExitAttributeMode>().unwrap();
    let vec = c.expand().to_vec().unwrap();
    println!("{vec:?}");
    println!("{}", String::from_utf8(vec).unwrap());
}

fn debug_input() {
    use nixtui_core::input::InputParser;
    let mut parser = InputParser::from_env().unwrap();
    parser.push_default();
    let mut tty = std::fs::File::open("/dev/tty").unwrap();
    let mut buf = [0_u8; 100];
    loop {
        let read = tty.read(&mut buf).unwrap();
        let slice = &buf[0..read];
        let parsed = parser.parse(slice);
        println!("{:?}", slice);
        println!("{parsed:#?}");
    }
}

fn test_tty_expand() {
    let mut tty = Tty::new().unwrap();
    tty.enter_ca_mode().unwrap();
    tty.move_cursor(10, 0).unwrap();
    tty.write_all(b"Moved cursor to 10;0\r\n").unwrap();
    tty.write_all(b"Hiding cursor\r\n").unwrap();
    tty.cursor_invisible().unwrap();
    std::thread::sleep(Duration::from_secs(2));
    tty.write_all(b"Showing cursor\r\n").unwrap();
    tty.cursor_normal_visibility().unwrap();
    std::thread::sleep(Duration::from_secs(2));
    tty.bold().unwrap();
    tty.italics().unwrap();
    tty.underline().unwrap();
    tty.write_all(b"Applied modifiers\r\n").unwrap();
    std::thread::sleep(Duration::from_secs(2));
    tty.exit_attribute_modes().unwrap();
    tty.write_all(b"Modifiers reset\r\n").unwrap();
    std::thread::sleep(Duration::from_secs(3));
    tty.exit_ca_mode().unwrap();
}
