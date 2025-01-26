pub mod errors;

use errors::TtyCapabilityError;
use nix::libc::ioctl;
use nix::sys::termios::Termios;
use nix::{
    libc::{VMIN, VTIME},
    sys::termios::{
        tcgetattr, tcsetattr, ControlFlags, InputFlags, LocalFlags, OutputFlags, SetArg,
    },
};
use std::io::{Read, Write};
use std::{
    fs::File,
    os::fd::{AsFd, AsRawFd},
};
use terminfo::capability as cap;

use crate::input::InputParser;

macro_rules! tty_expand_cap {
    ($db:expr, $write:expr, $path:path, $name:literal) => {
        {
            let Some(cap) = $db.get::<$path>() else {
                return Err(TtyCapabilityError::CapabilityNotFound { cap_name: $name.into() });
            };
            if let Err(err) = cap.expand().to($write) {
                use ::terminfo::Error as E;
                return match err {
                    E::Io(io_err) => Err(TtyCapabilityError::IoError(io_err)),
                    _ => Err(TtyCapabilityError::CapabilityExpansionError),
                }
            };
            Ok(())
        }
    };
    ($db:expr, $write:expr, $path:path, $name:literal; $first_param:expr $(,$params:expr)*$(,)?) => {
        {
            let Some(cap) = $db.get::<$path>() else {
                return Err(TtyCapabilityError::CapabilityNotFound { cap_name: $name.into() });
            };
            ::terminfo::expand!($write, cap.as_ref(); $first_param $(,$params)+ ).map_err(|e| {
                use ::terminfo::Error as E;
                match e {
                    E::Io(io_err) => TtyCapabilityError::IoError(io_err),
                    _ => TtyCapabilityError::CapabilityExpansionError,
                }
            })
        }
    }
}

pub struct Tty<IO: Write + Read + AsFd + 'static = File> {
    raw: IO,
    orig_termios: Termios,
    db: terminfo::Database,
}

impl<IO: Write + Read + AsFd + std::fmt::Debug + 'static> std::fmt::Debug for Tty<IO> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Tty {
            raw,
            orig_termios,
            db,
        } = self;
        f.debug_struct("Tty")
            .field("raw", raw)
            .field("orig_termios", orig_termios)
            .field("db", db)
            .finish()
    }
}

impl Tty<File> {
    pub fn new() -> Result<Self, errors::TtyCreationError> {
        let file = File::options().read(true).write(true).open("/dev/tty")?;
        let orig_termios = tcgetattr(file.as_fd())?;
        Ok(Self {
            raw: file,
            orig_termios,
            db: terminfo::Database::from_env()?,
        })
    }
}

impl<IO: Read + Write + AsFd> Tty<IO> {
    pub fn get_termios(&mut self) -> std::io::Result<Termios> {
        Ok(tcgetattr(self.raw.as_fd())?)
    }

    pub fn write_termios(&mut self, termios: Termios, mode: SetArg) -> std::io::Result<()> {
        tcsetattr(self.raw.as_fd(), mode, &termios)?;
        Ok(())
    }

    pub fn raw_mode(&mut self) -> std::io::Result<()> {
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

    /// Restores the original termios settings
    pub fn write_orig_termios(&mut self) -> std::io::Result<()> {
        Ok(tcsetattr(
            self.raw.as_fd(),
            SetArg::TCSAFLUSH,
            &self.orig_termios,
        )?)
    }

    pub fn read_u8(&mut self) -> std::io::Result<u8> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    pub fn move_cursor(&mut self, row: usize, col: usize) -> Result<(), TtyCapabilityError> {
        tty_expand_cap!(&self.db, &mut self.raw, cap::CursorAddress, "CursorAddress"; row as i32, col as i32)
    }

    pub fn cursor_invisible(&mut self) -> Result<(), TtyCapabilityError> {
        tty_expand_cap!(
            self.db,
            &mut self.raw,
            cap::CursorInvisible,
            "CursorInvisible"
        )
    }

    pub fn cursor_very_visible(&mut self) -> Result<(), TtyCapabilityError> {
        tty_expand_cap!(self.db, &mut self.raw, cap::CursorVisible, "CursorVisible")
    }

    pub fn cursor_normal_visibility(&mut self) -> Result<(), TtyCapabilityError> {
        tty_expand_cap!(self.db, &mut self.raw, cap::CursorNormal, "CursorNormal")
    }

    pub fn clean(&mut self) -> Result<(), TtyCapabilityError> {
        tty_expand_cap!(self.db, &mut self.raw, cap::ClearScreen, "ClearScreen")
    }

    pub fn bell(&mut self) -> Result<(), TtyCapabilityError> {
        tty_expand_cap!(self.db, &mut self.raw, cap::Bell, "Bell")
    }

    /// Turns on underline mode
    pub fn underline(&mut self) -> Result<(), TtyCapabilityError> {
        tty_expand_cap!(
            self.db,
            &mut self.raw,
            cap::EnterUnderlineMode,
            "EnterUnderlineMode"
        )
    }

    // Exits underline mode
    pub fn exit_underline(&mut self) -> Result<(), TtyCapabilityError> {
        tty_expand_cap!(
            self.db,
            &mut self.raw,
            cap::ExitUnderlineMode,
            "ExitEnderlineMode"
        )
    }

    /// Turns on italics mode
    pub fn italics(&mut self) -> Result<(), TtyCapabilityError> {
        tty_expand_cap!(
            self.db,
            &mut self.raw,
            cap::EnterItalicsMode,
            "EnterItalicsMode"
        )
    }

    /// Exits italics mode
    pub fn exit_italics(&mut self) -> Result<(), TtyCapabilityError> {
        tty_expand_cap!(
            self.db,
            &mut self.raw,
            cap::ExitItalicsMode,
            "ExitItalicsMode"
        )
    }

    /// Turns on bold mode
    pub fn bold(&mut self) -> Result<(), TtyCapabilityError> {
        tty_expand_cap!(self.db, &mut self.raw, cap::EnterBoldMode, "EnterBoldMode")
    }

    pub fn reverse(&mut self) -> Result<(), TtyCapabilityError> {
        tty_expand_cap!(
            self.db,
            &mut self.raw,
            cap::EnterReverseMode,
            "EnterReverseMode"
        )
    }

    /// Exits all atribute modes, e. g. `self.bold()`
    pub fn exit_attribute_modes(&mut self) -> Result<(), TtyCapabilityError> {
        tty_expand_cap!(
            self.db,
            &mut self.raw,
            cap::ExitAttributeMode,
            "ExitAttributeMode"
        )
    }

    pub fn enter_secure_mode(&mut self) -> Result<(), TtyCapabilityError> {
        tty_expand_cap!(
            self.db,
            &mut self.raw,
            cap::EnterSecureMode,
            "EnterSecureMode"
        )
    }

    pub fn enter_ca_mode(&mut self) -> Result<(), TtyCapabilityError> {
        tty_expand_cap!(self.db, &mut self.raw, cap::EnterCaMode, "EnterCaMode")
    }

    pub fn exit_ca_mode(&mut self) -> Result<(), TtyCapabilityError> {
        tty_expand_cap!(self.db, &mut self.raw, cap::ExitCaMode, "ExitCaMode")
    }

    pub fn size(&mut self) -> std::io::Result<Winsize> {
        let mut buf = nix::libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let ioctl_return = unsafe {
            ioctl(
                self.raw.as_fd().as_raw_fd(),
                nix::libc::TIOCGWINSZ,
                &mut buf,
            )
        };
        if ioctl_return < 0 {
            return Err(nix::errno::Errno::last().into());
        }

        Ok(buf.into())
    }

    pub fn set_bg_16(&mut self, color: usize) -> std::io::Result<()> {
        assert!(color < 8, "Valid color codes are in 0..8");
        let buf = format!("\x1B[{}m", 40 + color);
        self.raw.write_all(buf.as_bytes())?;
        Ok(())
    }

    pub fn make_parser(&self) -> InputParser {
        InputParser::from_terminfo(&self.db)
    }
}

pub struct Winsize {
    pub col: u16,
    pub row: u16,
}

impl From<nix::libc::winsize> for Winsize {
    fn from(value: nix::libc::winsize) -> Self {
        Self {
            col: value.ws_col,
            row: value.ws_row,
        }
    }
}

impl<IO: Read + Write + AsFd> Drop for Tty<IO> {
    fn drop(&mut self) {
        let _ = self.write_orig_termios();
    }
}

impl<IO: Read + Write + AsFd> Write for Tty<IO> {
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

impl<IO: Read + Write + AsFd> std::io::Read for Tty<IO> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.raw.read(buf)
    }
}
