mod changes;

use std::error::Error;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::{fs::File, os::fd::AsFd};

use changes::{FromTerminfo, TtyChange};
use nix::libc::ioctl;
use nix::sys::termios::Termios;
use nix::{
    libc::{VMIN, VTIME},
    sys::termios::{
        tcgetattr, tcsetattr, ControlFlags, InputFlags, LocalFlags, OutputFlags, SetArg,
    },
};

pub type Result<T> = std::result::Result<T, TtyError>;

pub struct Tty {
    raw: File,
    orig_termios: Termios,
    changes: Vec<Box<dyn TtyChange>>,
    db: terminfo::Database,
}

impl std::fmt::Debug for Tty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Tty {
            raw,
            orig_termios,
            changes: _,
            db,
        } = self;
        f.debug_struct("Tty")
            .field("raw", raw)
            .field("orig_termios", orig_termios)
            .field("db", db)
            .finish()
    }
}

impl Tty {
    pub fn new() -> Result<Self> {
        let file = File::options().read(true).write(true).open("/dev/tty")?;
        let orig_termios = tcgetattr(file.as_fd())?;
        Ok(Self {
            raw: file,
            orig_termios,
            changes: Vec::new(),
            db: terminfo::Database::from_env()?,
        })
    }

    pub fn apply_change_from_terminfo<C: TtyChange + FromTerminfo + 'static>(
        &mut self,
    ) -> Result<Option<()>> {
        let change = match C::from_terminfo(&self.db) {
            Some(v) => v,
            None => return Ok(None),
        };
        self.apply_change(change)?;
        Ok(Some(()))
    }

    pub fn apply_change<C: TtyChange + 'static>(&mut self, change: C) -> Result<()> {
        change.apply(&mut self.raw)?;
        self.changes.push(Box::new(change));
        Ok(())
    }

    pub fn get_termios(&mut self) -> Result<Termios> {
        Ok(tcgetattr(self.raw.as_fd())?)
    }

    pub fn write_termios(&mut self, termios: Termios, mode: SetArg) -> Result<()> {
        tcsetattr(self.raw.as_fd(), mode, &termios)?;
        Ok(())
    }

    pub fn raw_mode(&mut self) -> Result<()> {
        let ttyfd = self.raw.as_fd();
        let mut termios = self.orig_termios.clone();
        // According to https://www.man7.org/linux/man-pages/man3/termios.3.html `Raw mode` section
        {
            termios.input_flags &= !(InputFlags::IGNBRK
                | InputFlags::BRKINT
                | InputFlags::PARMRK
                | InputFlags::ISTRIP
                | InputFlags::ICRNL
                | InputFlags::IGNCR
                | InputFlags::ICRNL
                | InputFlags::IXON);

            termios.output_flags &= !OutputFlags::OPOST;

            termios.local_flags &= !(LocalFlags::ECHO
                | LocalFlags::ECHONL
                | LocalFlags::ICANON
                | LocalFlags::ISIG
                | LocalFlags::IEXTEN);

            termios.control_flags &= !(ControlFlags::CSIZE | ControlFlags::PARENB);
            termios.control_flags |= ControlFlags::CS8;
            termios.control_chars[VTIME] = 0;
            termios.control_chars[VMIN] = 1;
        }
        tcsetattr(ttyfd, SetArg::TCSAFLUSH, &termios)?;
        Ok(())
    }

    /// Saves the current cursor position and restores it when
    /// Tty is being dropped
    pub fn c_save_cursor(&mut self) -> Result<Option<()>> {
        self.apply_change_from_terminfo::<changes::SaveCursor>()
    }

    /// Enters the ca mode and exites it when Tty is being dropped
    pub fn c_enter_ca_mode(&mut self) -> Result<Option<()>> {
        self.apply_change_from_terminfo::<changes::EnterCaMode>()
    }

    /// Restores the original termios settings
    pub fn write_orig_termios(&mut self) -> Result<()> {
        Ok(tcsetattr(
            self.raw.as_fd(),
            SetArg::TCSAFLUSH,
            &self.orig_termios,
        )?)
    }

    pub fn revert_changes(&mut self) -> Result<()> {
        self.write_orig_termios()?;
        for c in self.changes.iter() {
            c.revert(&mut self.raw)?;
        }
        Ok(())
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    pub fn move_cursor(&mut self, row: usize, col: usize) -> Result<()> {
        let buf = format!("\x1B[{};{}H", row + 1, col + 1);
        self.write_all(buf.as_bytes())?;
        Ok(())
    }

    pub fn hide_cursor(&mut self) -> Result<()> {
        self.write_all(b"\x1B[?25l")?;
        Ok(())
    }

    pub fn show_cursor(&mut self) -> Result<()> {
        self.write_all(b"\x1B[?25h")?;
        Ok(())
    }

    pub fn clean(&mut self) -> Result<()> {
        self.write_all(b"\x1B[2J")?;
        Ok(())
    }

    pub fn size(&mut self) -> Result<nix::libc::winsize> {
        let mut buf = nix::libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        unsafe { ioctl(self.raw.as_raw_fd(), nix::libc::TIOCGWINSZ, &mut buf) };
        // todo! handle possible errors created by ioctl
        Ok(buf)
    }

    pub fn set_bg_16(&mut self, color: usize) -> Result<()> {
        assert!(color < 8, "Valid color codes are in 0..8");
        let buf = format!("\x1B[{}m", 40 + color);
        self.raw.write_all(buf.as_bytes())?;
        Ok(())
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.raw.read_exact(buf)?;
        Ok(())
    }
}

impl Drop for Tty {
    fn drop(&mut self) {
        let _ = self.revert_changes();
    }
}

impl Write for Tty {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.raw.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.raw.flush()
    }
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
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
pub struct TtyError {
    pub kind: Kind,
    pub value: Box<dyn Error + Send + Sync>,
}

#[derive(Debug)]
pub enum Kind {
    IOError,
    TerminfoError,
    Errno,
}

impl From<std::io::Error> for TtyError {
    fn from(val: std::io::Error) -> Self {
        TtyError {
            kind: Kind::IOError,
            value: Box::new(val),
        }
    }
}

impl From<terminfo::Error> for TtyError {
    fn from(value: terminfo::Error) -> Self {
        TtyError {
            kind: Kind::TerminfoError,
            value: Box::new(value),
        }
    }
}

impl From<nix::errno::Errno> for TtyError {
    fn from(value: nix::errno::Errno) -> Self {
        TtyError {
            kind: Kind::Errno,
            value: Box::new(value),
        }
    }
}
