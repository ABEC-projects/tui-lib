use std::io::Read;

fn main() {
    get_cap();
}

fn get_cap() {
    use terminfo::capability as cap;
    let db = terminfo::Database::from_env().unwrap();
    let c = db.get::<cap::FlashScreen>().unwrap();
    let vec = c.expand().to_vec().unwrap();
    println!("{vec:?}");
    println!("{}", String::from_utf8(vec).unwrap());
}

fn debug_input() {
   use nixtui::input::InputParser;
   let mut parser = InputParser::from_env().unwrap();
   parser.push_default();
   let mut tty = std::fs::File::open("/dev/tty").unwrap();
   let mut buf = [0_u8;100];
   loop {
       let read = tty.read(&mut buf).unwrap();
       let slice = &buf[0..read];
       let parsed = parser.parse(slice);
       println!("{:?}", slice);
       println!("{parsed:#?}");
   }
}
