use std::io::Read;

fn main() {
   use nixtui::tty::input::InputParser;
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

