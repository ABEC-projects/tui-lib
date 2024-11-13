use std::io::Write;

use tui::{tty::Tty, ui::Anchor};
use ::tui::ui::Tui;

fn main() {
    let mut tty = Tty::new().unwrap();
    let mut ui = Tui::new(tty.size().unwrap().into());
    let upper_left = ui.add_anchor(Anchor::new_abs_from_upper_left(5, 5));
    let down_right = ui.add_anchor(Anchor::new_rel_from_down_right(0.5, 0.5));
    let rect = ui.add_rect(&upper_left, &down_right);
    let anchor_ = Anchor::new_rel_from_down_right(0.5, 0.5);
    let anchor = ui.add_anchor_in(anchor_, &rect);

    loop {
        ui.update_size(tty.size().unwrap().into());
        tty.clean().unwrap();
        let cords = ui.get_cords_of_anchor(&anchor);
        tty.move_cursor(cords.row, cords.col).unwrap();
        tty.write_all(b"@").unwrap();
        let cords = ui.get_cords_of_anchor(&down_right);
        tty.move_cursor(cords.row, cords.col).unwrap();
        tty.write_all(b"*").unwrap();
        let cords = ui.get_cords_of_anchor(&upper_left);
        tty.move_cursor(cords.row, cords.col).unwrap();
        tty.write_all(b"*").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
