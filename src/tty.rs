use std::error::Error;
use std::os::fd::AsRawFd;
use std::{fs::File, os::fd::AsFd};
use std::io::{Read, Write};

use nix::libc::ioctl;
use nix::sys::termios::Termios;
use nix::{libc::{VMIN, VTIME}, sys::termios::{tcgetattr, tcsetattr, ControlFlags, InputFlags, LocalFlags, OutputFlags, SetArg}};

pub type Result<T> = std::result::Result<T, TtyError>;


#[derive(Debug)]
pub struct Tty  {
    pub raw: File,
    orig_termios: Termios,
}

impl Tty {
    
    pub fn new() -> Result<Self> {
        let file = File::options()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .unwrap();
        let orig_termios = tcgetattr(file.as_fd()).unwrap();
        Ok(Self { raw: file, orig_termios })
    }

    pub fn get_termios(&mut self) -> Result<Termios> {
        Ok(tcgetattr(self.raw.as_fd()).unwrap())
    }

    pub fn write_termios(&mut self, termios: Termios, mode: SetArg) -> Result<()> {
        tcsetattr(self.raw.as_fd(), mode, &termios).unwrap();
        Ok(())
    }


    pub fn uncook (&mut self) -> Result<()> {
        let ttyfd = self.raw.as_fd();
        let mut termios = self.orig_termios.clone();
        // According to https://www.man7.org/linux/man-pages/man3/termios.3.html `Raw mode` section
        {
            termios.input_flags &= !(
                InputFlags::IGNBRK
                | InputFlags::BRKINT
                | InputFlags::PARMRK
                | InputFlags::ISTRIP
                | InputFlags::ICRNL 
                | InputFlags::IGNCR
                | InputFlags::ICRNL
                | InputFlags::IXON 
            );

            termios.output_flags &= !OutputFlags::OPOST;

            termios.local_flags &= !(
                LocalFlags::ECHO
                | LocalFlags::ECHONL
                | LocalFlags::ICANON 
                | LocalFlags::ISIG
                | LocalFlags::IEXTEN
            );

            termios.control_flags &= !(
                ControlFlags::CSIZE
                | ControlFlags::PARENB
            );
            termios.control_flags |= ControlFlags::CS8;
            termios.control_chars[VTIME] = 0;
            termios.control_chars[VMIN] = 1;
        }
        tcsetattr(ttyfd, SetArg::TCSAFLUSH, &termios).unwrap();
        // todo! change to use terminfo
        self.raw.write_all(b"\x1B[?5l").unwrap();
        self.raw.write_all(b"\x1B[s").unwrap();
        self.raw.write_all(b"\x1B[?47h").unwrap();
        self.raw.write_all(b"\x1B[?1049h").unwrap();
        Ok(())
    }

    pub fn recook (&mut self) -> Result<()>{
        tcsetattr(self.raw.as_fd(), SetArg::TCSAFLUSH, &self.orig_termios).unwrap();
        self.raw.write_all(b"\x1B[?1049l").unwrap();
        self.raw.write_all(b"\x1B[?47l").unwrap();
        self.raw.write_all(b"\x1B[u").unwrap();
        Ok(())
    }

    pub fn read_u8 (&mut self) -> Result<u8> {
        let mut buf = [0;1];
        self.raw.read_exact(&mut buf).unwrap();
        Ok(buf[0])
    }

    pub fn move_cursor(&mut self, row: usize, col: usize) -> Result<()> {
        let buf = format!("\x1B[{};{}H", row + 1, col + 1);
        self.write_all(buf.as_bytes()).unwrap();
        Ok(())
    }
    
    pub fn hide_cursor (&mut self) -> Result<()>{
        self.write_all(b"\x1B[?25l").unwrap();
        Ok(())
    }

    pub fn show_cursor (&mut self) -> Result<()> {
        self.write_all(b"\x1B[?25h").unwrap();
        Ok(())
    }

    pub fn clean(&mut self) -> Result<()> {
        self.write_all(b"\x1B[2J").unwrap();
        Ok(())
    }

    pub fn size(&mut self) -> Result<nix::libc::winsize> {
        let mut buf = nix::libc::winsize{ws_row:0, ws_col:0, ws_xpixel:0, ws_ypixel:0};
        unsafe { ioctl(self.raw.as_raw_fd(), nix::libc::TIOCGWINSZ, &mut buf)};
        // todo! handle possible errors created by ioctl
        Ok(buf)
    }

    pub fn set_bg_16(&mut self, color: usize) -> Result<()>{
        assert!(color < 8, "Valid color codes are in 0..8");
        let buf = format!("\x1B[{}m", 40+color);
        self.raw.write_all(buf.as_bytes()).unwrap();
        Ok(())
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.raw.read_exact(buf).unwrap();
        Ok(())
    }
}

impl Write for Tty  {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.raw.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.raw.flush()
    }
    fn by_ref(&mut self) -> &mut Self
        where Self: Sized, 
    {
        self
    }
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.raw.write_all(buf)
    }
    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        self.raw.write_fmt(fmt)
    }
    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        self.raw.write_vectored(bufs)
    }
}

impl std::io::Read for Tty {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.raw.read(buf)
    }
}


#[derive(Debug)]
pub struct  TtyError {
    kind: Kind,
    value: Box<dyn Error + Send + Sync>
}

#[derive(Debug)]
pub enum Kind {
    IOError,
}

impl From<std::io::Error> for TtyError {
    fn from(val: std::io::Error) -> Self {
        TtyError {
            kind: Kind::IOError,
            value: Box::new(val)
        }
    }
}

