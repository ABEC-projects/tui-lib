pub mod errors;

use errors::CapabilityError;
use nix::libc::ioctl;
use nix::sys::termios::Termios;
use nix::{
    libc::{VMIN, VTIME},
    sys::termios::{
        tcgetattr, tcsetattr, ControlFlags, InputFlags, LocalFlags, OutputFlags, SetArg,
    },
};
use std::os::fd::{AsFd, AsRawFd};
use terminfo::{capability as cap, Capability, Database};

use crate::input::InputParser;
macro_rules! tty_expand_cap {
    ($db:expr, $to:expr, $cap:ty) => {
        {
            let Some(cap) = $db.get::<$cap>() else {
                return Err(CapabilityError::CapabilityNotFound { cap_name: <$cap>::name().into() });
            };
            ::terminfo::expand!($to, cap.as_ref()).map_err(|e| {
                use ::terminfo::Error as E;
                match e {
                    E::Io(io_err) => CapabilityError::IoError(io_err),
                    _ => CapabilityError::CapabilityExpansionError,
                }
            })
        }
    };
    ($db:expr, $to:expr, $cap:ty; $first_param:expr $(,$params:expr)*$(,)?) => {
        {
            let Some(cap) = $db.get::<$cap>() else {
                return Err(CapabilityError::CapabilityNotFound { cap_name: <$cap>::name().into() });
            };
            ::terminfo::expand!($to, cap.as_ref(); $first_param $(,$params)* ).map_err(|e| {
                use ::terminfo::Error as E;
                match e {
                    E::Io(io_err) => CapabilityError::IoError(io_err),
                    _ => CapabilityError::CapabilityExpansionError,
                }
            })
        }
    };
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

#[cfg(target_family = "unix")]
pub trait UnixTerminal: AsFd {
    fn get_termios(&mut self) -> std::io::Result<Termios>;
    fn set_termios(&mut self, termios: &Termios, mode: SetArg) -> std::io::Result<()>;
    fn raw_mode(&mut self) -> std::io::Result<()> {
        let mut termios = self.get_termios()?;
        let ttyfd = self.as_fd();
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
    fn get_size(&mut self) -> std::io::Result<Winsize> {
        let mut buf = nix::libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let ioctl_return =
            unsafe { ioctl(self.as_fd().as_raw_fd(), nix::libc::TIOCGWINSZ, &mut buf) };
        nix::errno::Errno::result(ioctl_return)?;

        Ok(buf.into())
    }
}

impl<T: AsFd> UnixTerminal for T {
    fn get_termios(&mut self) -> std::io::Result<Termios> {
        tcgetattr(self).map_err(|e| e.into())
    }
    fn set_termios(&mut self, termios: &Termios, mode: SetArg) -> std::io::Result<()> {
        tcsetattr(self, mode, termios).map_err(|e| e.into())
    }
}

pub struct TerminfoWrapper {
    pub db: Database,
    buffer: Vec<u8>,
}

impl<'a> TerminfoWrapper {
    pub fn from_env() -> Result<Self, errors::TerminfoCreationError> {
        Ok(Self {
            db: Database::from_env()?,
            buffer: Vec::new(),
        })
    }

    pub fn flush_to(&mut self, to: &mut impl std::io::Write) -> std::io::Result<()> {
        to.write_all(&self.buffer)?;
        self.clear();
        Ok(())
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn append(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    pub fn move_cursor(&mut self, row: usize, col: usize) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CursorAddress; row as i32, col as i32)
    }
    pub fn back_tab(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::BackTab)
    }
    pub fn bell(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::Bell)
    }
    pub fn carriage_return(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CarriageReturn)
    }
    pub fn clear_all_tabs(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ClearAllTabs)
    }
    pub fn clear_screen(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ClearScreen)
    }
    pub fn clr_eol(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ClrEol)
    }
    pub fn clr_eos(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ClrEos)
    }
    pub fn command_character(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CommandCharacter)
    }
    pub fn cursor_down(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CursorDown)
    }
    pub fn cursor_home(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CursorHome)
    }
    pub fn cursor_invisible(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CursorInvisible)
    }
    pub fn cursor_left(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CursorLeft)
    }
    pub fn cursor_mem_address(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CursorMemAddress)
    }
    pub fn cursor_normal(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CursorNormal)
    }
    pub fn cursor_right(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CursorRight)
    }
    pub fn cursor_to_ll(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CursorToLl)
    }
    pub fn cursor_up(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CursorUp)
    }
    pub fn cursor_visible(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CursorVisible)
    }
    pub fn delete_character(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::DeleteCharacter)
    }
    pub fn delete_line(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::DeleteLine)
    }
    pub fn dis_status_line(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::DisStatusLine)
    }
    pub fn down_half_line(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::DownHalfLine)
    }
    pub fn enter_alt_charset_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterAltCharsetMode)
    }
    pub fn enter_blink_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterBlinkMode)
    }
    pub fn enter_bold_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterBoldMode)
    }
    pub fn enter_ca_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterCaMode)
    }
    pub fn enter_delete_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterDeleteMode)
    }
    pub fn enter_dim_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterDimMode)
    }
    pub fn enter_insert_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterInsertMode)
    }
    pub fn enter_secure_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterSecureMode)
    }
    pub fn enter_protected_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterProtectedMode)
    }
    pub fn enter_reverse_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterReverseMode)
    }
    pub fn enter_standout_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterStandoutMode)
    }
    pub fn enter_underline_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterUnderlineMode)
    }
    pub fn exit_alt_charset_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitAltCharsetMode)
    }
    pub fn exit_attribute_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitAttributeMode)
    }
    pub fn exit_ca_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitCaMode)
    }
    pub fn exit_delete_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitDeleteMode)
    }
    pub fn exit_insert_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitInsertMode)
    }
    pub fn exit_standout_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitStandoutMode)
    }
    pub fn exit_underline_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitUnderlineMode)
    }
    pub fn flash_screen(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::FlashScreen)
    }
    pub fn form_feed(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::FormFeed)
    }
    pub fn from_status_line(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::FromStatusLine)
    }
    pub fn init_1string(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::Init1String)
    }
    pub fn init_2string(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::Init2String)
    }
    pub fn init_3string(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::Init3String)
    }
    pub fn init_file(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::InitFile)
    }
    pub fn insert_character(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::InsertCharacter)
    }
    pub fn insert_line(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::InsertLine)
    }
    pub fn insert_padding(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::InsertPadding)
    }
    pub fn key_backspace(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyBackspace)
    }
    pub fn key_catab(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyCATab)
    }
    pub fn key_clear(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyClear)
    }
    pub fn key_ctab(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyCTab)
    }
    pub fn key_dc(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyDc)
    }
    pub fn key_dl(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyDl)
    }
    pub fn key_down(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyDown)
    }
    pub fn key_eic(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyEic)
    }
    pub fn key_eol(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyEol)
    }
    pub fn key_eos(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyEos)
    }
    pub fn key_f0(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF0)
    }
    pub fn key_f1(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF1)
    }
    pub fn key_f10(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF10)
    }
    pub fn key_f2(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF2)
    }
    pub fn key_f3(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF3)
    }
    pub fn key_f4(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF4)
    }
    pub fn key_f5(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF5)
    }
    pub fn key_f6(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF6)
    }
    pub fn key_f7(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF7)
    }
    pub fn key_f8(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF8)
    }
    pub fn key_f9(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF9)
    }
    pub fn key_home(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyHome)
    }
    pub fn key_ic(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyIc)
    }
    pub fn key_il(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyIl)
    }
    pub fn key_left(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyLeft)
    }
    pub fn key_ll(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyLl)
    }
    pub fn key_npage(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyNPage)
    }
    pub fn key_ppage(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyPPage)
    }
    pub fn key_right(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyRight)
    }
    pub fn key_sf(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySf)
    }
    pub fn key_sr(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySr)
    }
    pub fn key_stab(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySTab)
    }
    pub fn key_up(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyUp)
    }
    pub fn keypad_local(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeypadLocal)
    }
    pub fn keypad_xmit(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeypadXmit)
    }
    pub fn lab_f0(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::LabF0)
    }
    pub fn lab_f1(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::LabF1)
    }
    pub fn lab_f10(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::LabF10)
    }
    pub fn lab_f2(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::LabF2)
    }
    pub fn lab_f3(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::LabF3)
    }
    pub fn lab_f4(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::LabF4)
    }
    pub fn lab_f5(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::LabF5)
    }
    pub fn lab_f6(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::LabF6)
    }
    pub fn lab_f7(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::LabF7)
    }
    pub fn lab_f8(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::LabF8)
    }
    pub fn lab_f9(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::LabF9)
    }
    pub fn meta_off(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::MetaOff)
    }
    pub fn meta_on(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::MetaOn)
    }
    pub fn newline(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::Newline)
    }
    pub fn pad_char(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::PadChar)
    }
    pub fn pkey_key(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::PKeyKey)
    }
    pub fn pkey_local(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::PKeyLocal)
    }
    pub fn pkey_xmit(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::PKeyXmit)
    }
    pub fn print_screen(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::PrintScreen)
    }
    pub fn prtr_off(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::PrtrOff)
    }
    pub fn prtr_on(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::PrtrOn)
    }
    pub fn repeat_char(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::RepeatChar)
    }
    pub fn reset_1string(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::Reset1String)
    }
    pub fn reset_2string(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::Reset2String)
    }
    pub fn reset_3string(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::Reset3String)
    }
    pub fn reset_file(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ResetFile)
    }
    pub fn restore_cursor(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::RestoreCursor)
    }
    pub fn save_cursor(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SaveCursor)
    }
    pub fn scroll_forward(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ScrollForward)
    }
    pub fn scroll_reverse(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ScrollReverse)
    }
    pub fn set_tab(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetTab)
    }
    pub fn set_window(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetWindow)
    }
    pub fn tab(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::Tab)
    }
    pub fn to_status_line(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ToStatusLine)
    }
    pub fn underline_char(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::UnderlineChar)
    }
    pub fn up_half_line(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::UpHalfLine)
    }
    pub fn init_prog(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::InitProg)
    }
    pub fn key_a1(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyA1)
    }
    pub fn key_a3(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyA3)
    }
    pub fn key_b2(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyB2)
    }
    pub fn key_c1(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyC1)
    }
    pub fn key_c3(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyC3)
    }
    pub fn prtr_non(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::PrtrNon)
    }
    pub fn char_padding(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CharPadding)
    }
    pub fn acs_chars(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::AcsChars)
    }
    pub fn plab_norm(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::PlabNorm)
    }
    pub fn key_btab(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyBTab)
    }
    pub fn enter_xon_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterXonMode)
    }
    pub fn exit_xon_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitXonMode)
    }
    pub fn enter_am_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterAmMode)
    }
    pub fn exit_am_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitAmMode)
    }
    pub fn xon_character(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::XonCharacter)
    }
    pub fn xoff_character(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::XoffCharacter)
    }
    pub fn ena_acs(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnaAcs)
    }
    pub fn label_on(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::LabelOn)
    }
    pub fn label_off(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::LabelOff)
    }
    pub fn key_beg(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyBeg)
    }
    pub fn key_cancel(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyCancel)
    }
    pub fn key_close(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyClose)
    }
    pub fn key_command(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyCommand)
    }
    pub fn key_copy(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyCopy)
    }
    pub fn key_create(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyCreate)
    }
    pub fn key_end(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyEnd)
    }
    pub fn key_enter(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyEnter)
    }
    pub fn key_exit(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyExit)
    }
    pub fn key_find(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyFind)
    }
    pub fn key_help(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyHelp)
    }
    pub fn key_mark(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyMark)
    }
    pub fn key_message(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyMessage)
    }
    pub fn key_move(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyMove)
    }
    pub fn key_next(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyNext)
    }
    pub fn key_open(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyOpen)
    }
    pub fn key_options(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyOptions)
    }
    pub fn key_previous(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyPrevious)
    }
    pub fn key_print(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyPrint)
    }
    pub fn key_redo(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyRedo)
    }
    pub fn key_reference(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyReference)
    }
    pub fn key_refresh(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyRefresh)
    }
    pub fn key_replace(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyReplace)
    }
    pub fn key_restart(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyRestart)
    }
    pub fn key_resume(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyResume)
    }
    pub fn key_save(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySave)
    }
    pub fn key_suspend(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySuspend)
    }
    pub fn key_undo(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyUndo)
    }
    pub fn key_sbeg(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySBeg)
    }
    pub fn key_scancel(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySCancel)
    }
    pub fn key_scommand(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySCommand)
    }
    pub fn key_scopy(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySCopy)
    }
    pub fn key_screate(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySCreate)
    }
    pub fn key_sdc(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySDc)
    }
    pub fn key_sdl(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySDl)
    }
    pub fn key_select(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySelect)
    }
    pub fn key_send(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySEnd)
    }
    pub fn key_seol(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySEol)
    }
    pub fn key_sexit(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySExit)
    }
    pub fn key_sfind(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySFind)
    }
    pub fn key_shelp(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySHelp)
    }
    pub fn key_shome(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySHome)
    }
    pub fn key_sic(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySIc)
    }
    pub fn key_sleft(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySLeft)
    }
    pub fn key_smessage(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySMessage)
    }
    pub fn key_smove(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySMove)
    }
    pub fn key_snext(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySNext)
    }
    pub fn key_soptions(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySOptions)
    }
    pub fn key_sprevious(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySPrevious)
    }
    pub fn key_sprint(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySPrint)
    }
    pub fn key_sredo(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySRedo)
    }
    pub fn key_sreplace(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySReplace)
    }
    pub fn key_sright(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySRight)
    }
    pub fn key_srsume(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySRsume)
    }
    pub fn key_ssave(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySSave)
    }
    pub fn key_ssuspend(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySSuspend)
    }
    pub fn key_sundo(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeySUndo)
    }
    pub fn req_for_input(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ReqForInput)
    }
    pub fn key_f11(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF11)
    }
    pub fn key_f12(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF12)
    }
    pub fn key_f13(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF13)
    }
    pub fn key_f14(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF14)
    }
    pub fn key_f15(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF15)
    }
    pub fn key_f16(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF16)
    }
    pub fn key_f17(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF17)
    }
    pub fn key_f18(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF18)
    }
    pub fn key_f19(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF19)
    }
    pub fn key_f20(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF20)
    }
    pub fn key_f21(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF21)
    }
    pub fn key_f22(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF22)
    }
    pub fn key_f23(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF23)
    }
    pub fn key_f24(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF24)
    }
    pub fn key_f25(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF25)
    }
    pub fn key_f26(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF26)
    }
    pub fn key_f27(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF27)
    }
    pub fn key_f28(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF28)
    }
    pub fn key_f29(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF29)
    }
    pub fn key_f30(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF30)
    }
    pub fn key_f31(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF31)
    }
    pub fn key_f32(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF32)
    }
    pub fn key_f33(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF33)
    }
    pub fn key_f34(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF34)
    }
    pub fn key_f35(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF35)
    }
    pub fn key_f36(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF36)
    }
    pub fn key_f37(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF37)
    }
    pub fn key_f38(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF38)
    }
    pub fn key_f39(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF39)
    }
    pub fn key_f40(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF40)
    }
    pub fn key_f41(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF41)
    }
    pub fn key_f42(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF42)
    }
    pub fn key_f43(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF43)
    }
    pub fn key_f44(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF44)
    }
    pub fn key_f45(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF45)
    }
    pub fn key_f46(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF46)
    }
    pub fn key_f47(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF47)
    }
    pub fn key_f48(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF48)
    }
    pub fn key_f49(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF49)
    }
    pub fn key_f50(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF50)
    }
    pub fn key_f51(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF51)
    }
    pub fn key_f52(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF52)
    }
    pub fn key_f53(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF53)
    }
    pub fn key_f54(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF54)
    }
    pub fn key_f55(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF55)
    }
    pub fn key_f56(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF56)
    }
    pub fn key_f57(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF57)
    }
    pub fn key_f58(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF58)
    }
    pub fn key_f59(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF59)
    }
    pub fn key_f60(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF60)
    }
    pub fn key_f61(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF61)
    }
    pub fn key_f62(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF62)
    }
    pub fn key_f63(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyF63)
    }
    pub fn clr_bol(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ClrBol)
    }
    pub fn clear_margins(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ClearMargins)
    }
    pub fn set_left_margin(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetLeftMargin)
    }
    pub fn set_right_margin(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetRightMargin)
    }
    pub fn label_format(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::LabelFormat)
    }
    pub fn set_clock(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetClock)
    }
    pub fn display_clock(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::DisplayClock)
    }
    pub fn remove_clock(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::RemoveClock)
    }
    pub fn create_window(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CreateWindow)
    }
    pub fn goto_window(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::GotoWindow)
    }
    pub fn hangup(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::Hangup)
    }
    pub fn dial_phone(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::DialPhone)
    }
    pub fn quick_dial(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::QuickDial)
    }
    pub fn tone(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::Tone)
    }
    pub fn pulse(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::Pulse)
    }
    pub fn flash_hook(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::FlashHook)
    }
    pub fn fixed_pause(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::FixedPause)
    }
    pub fn wait_tone(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::WaitTone)
    }
    pub fn user0(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::User0)
    }
    pub fn user1(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::User1)
    }
    pub fn user2(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::User2)
    }
    pub fn user3(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::User3)
    }
    pub fn user4(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::User4)
    }
    pub fn user5(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::User5)
    }
    pub fn user6(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::User6)
    }
    pub fn user7(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::User7)
    }
    pub fn user8(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::User8)
    }
    pub fn user9(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::User9)
    }
    pub fn orig_pair(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::OrigPair)
    }
    pub fn orig_colors(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::OrigColors)
    }
    pub fn initialize_color(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::InitializeColor)
    }
    pub fn initialize_pair(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::InitializePair)
    }
    pub fn set_color_pair(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetColorPair)
    }
    pub fn change_char_pitch(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ChangeCharPitch)
    }
    pub fn change_line_pitch(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ChangeLinePitch)
    }
    pub fn change_res_horz(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ChangeResHorz)
    }
    pub fn change_res_vert(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ChangeResVert)
    }
    pub fn define_char(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::DefineChar)
    }
    pub fn enter_doublewide_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterDoublewideMode)
    }
    pub fn enter_draft_quality(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterDraftQuality)
    }
    pub fn enter_italics_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterItalicsMode)
    }
    pub fn enter_leftward_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterLeftwardMode)
    }
    pub fn enter_micro_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterMicroMode)
    }
    pub fn enter_near_letter_quality(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterNearLetterQuality)
    }
    pub fn enter_normal_quality(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterNormalQuality)
    }
    pub fn enter_shadow_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterShadowMode)
    }
    pub fn enter_subscript_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterSubscriptMode)
    }
    pub fn enter_superscript_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterSuperscriptMode)
    }
    pub fn enter_upward_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterUpwardMode)
    }
    pub fn exit_doublewide_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitDoublewideMode)
    }
    pub fn exit_italics_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitItalicsMode)
    }
    pub fn exit_leftward_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitLeftwardMode)
    }
    pub fn exit_micro_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitMicroMode)
    }
    pub fn exit_shadow_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitShadowMode)
    }
    pub fn exit_subscript_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitSubscriptMode)
    }
    pub fn exit_superscript_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitSuperscriptMode)
    }
    pub fn exit_upward_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitUpwardMode)
    }
    pub fn micro_column_address(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::MicroColumnAddress)
    }
    pub fn micro_down(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::MicroDown)
    }
    pub fn micro_left(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::MicroLeft)
    }
    pub fn micro_right(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::MicroRight)
    }
    pub fn micro_row_address(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::MicroRowAddress)
    }
    pub fn micro_up(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::MicroUp)
    }
    pub fn order_of_pins(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::OrderOfPins)
    }
    pub fn select_char_set(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SelectCharSet)
    }
    pub fn set_bottom_margin(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetBottomMargin)
    }
    pub fn set_bottom_margin_parm(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetBottomMarginParm)
    }
    pub fn set_left_margin_parm(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetLeftMarginParm)
    }
    pub fn set_right_margin_parm(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetRightMarginParm)
    }
    pub fn set_top_margin(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetTopMargin)
    }
    pub fn set_top_margin_parm(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetTopMarginParm)
    }
    pub fn start_bit_image(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::StartBitImage)
    }
    pub fn start_char_set_def(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::StartCharSetDef)
    }
    pub fn stop_bit_image(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::StopBitImage)
    }
    pub fn stop_char_set_def(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::StopCharSetDef)
    }
    pub fn subscript_characters(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SubscriptCharacters)
    }
    pub fn superscript_characters(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SuperscriptCharacters)
    }
    pub fn these_cause_cr(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::TheseCauseCr)
    }
    pub fn zero_motion(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ZeroMotion)
    }
    pub fn char_set_names(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CharSetNames)
    }
    pub fn key_mouse(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::KeyMouse)
    }
    pub fn mouse_info(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::MouseInfo)
    }
    pub fn req_mouse_pos(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ReqMousePos)
    }
    pub fn get_mouse(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::GetMouse)
    }
    pub fn pkey_plab(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::PkeyPlab)
    }
    pub fn device_type(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::DeviceType)
    }
    pub fn code_set_init(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CodeSetInit)
    }
    pub fn set0_des_seq(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::Set0DesSeq)
    }
    pub fn set1_des_seq(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::Set1DesSeq)
    }
    pub fn set2_des_seq(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::Set2DesSeq)
    }
    pub fn set3_des_seq(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::Set3DesSeq)
    }
    pub fn set_lr_margin(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetLrMargin)
    }
    pub fn set_tb_margin(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetTbMargin)
    }
    pub fn bit_image_repeat(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::BitImageRepeat)
    }
    pub fn bit_image_newline(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::BitImageNewline)
    }
    pub fn bit_image_carriage_return(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::BitImageCarriageReturn)
    }
    pub fn color_names(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ColorNames)
    }
    pub fn define_bit_image_region(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::DefineBitImageRegion)
    }
    pub fn end_bit_image_region(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EndBitImageRegion)
    }
    pub fn set_color_band(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetColorBand)
    }
    pub fn set_page_length(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetPageLength)
    }
    pub fn display_pc_char(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::DisplayPcChar)
    }
    pub fn enter_pc_charset_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterPcCharsetMode)
    }
    pub fn exit_pc_charset_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitPcCharsetMode)
    }
    pub fn enter_scancode_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterScancodeMode)
    }
    pub fn exit_scancode_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ExitScancodeMode)
    }
    pub fn pc_term_options(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::PcTermOptions)
    }
    pub fn scancode_escape(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ScancodeEscape)
    }
    pub fn alt_scancode_esc(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::AltScancodeEsc)
    }
    pub fn enter_horizontal_hl_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterHorizontalHlMode)
    }
    pub fn enter_left_hl_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterLeftHlMode)
    }
    pub fn enter_low_hl_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterLowHlMode)
    }
    pub fn enter_right_hl_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterRightHlMode)
    }
    pub fn enter_top_hl_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterTopHlMode)
    }
    pub fn enter_vertical_hl_mode(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EnterVerticalHlMode)
    }
    pub fn set_a_attributes(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetAAttributes)
    }
    pub fn set_pglen_inch(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetPglenInch)
    }
    pub fn termcap_init2(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::TermcapInit2)
    }
    pub fn termcap_reset(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::TermcapReset)
    }
    pub fn linefeed_if_not_lf(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::LinefeedIfNotLf)
    }
    pub fn backspace_if_not_bs(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::BackspaceIfNotBs)
    }
    pub fn other_non_function_keys(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::OtherNonFunctionKeys)
    }
    pub fn arrow_key_map(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ArrowKeyMap)
    }
    pub fn acs_ulcorner(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::AcsULcorner)
    }
    pub fn acs_llcorner(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::AcsLLcorner)
    }
    pub fn acs_urcorner(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::AcsURcorner)
    }
    pub fn acs_lrcorner(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::AcsLRcorner)
    }
    pub fn acs_ltee(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::AcsLTee)
    }
    pub fn acs_rtee(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::AcsRTee)
    }
    pub fn acs_btee(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::AcsBTee)
    }
    pub fn acs_ttee(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::AcsTTee)
    }
    pub fn acs_hline(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::AcsHLine)
    }
    pub fn acs_vline(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::AcsVLine)
    }
    pub fn acs_plus(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::AcsPlus)
    }
    pub fn memory_lock(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::MemoryLock)
    }
    pub fn memory_unlock(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::MemoryUnlock)
    }
    pub fn box_chars_1(&mut self) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::BoxChars1)
    }

    pub fn expand_write<C>(&'a mut self) -> Result<(), CapabilityError>
    where
        C: terminfo::Capability<'a> + AsRef<[u8]>,
    {
        tty_expand_cap!(self.db, &mut self.buffer, C)
    }

    pub fn change_scroll_region(
        &mut self,
        top: u32,
        bottom: u32,
    ) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ChangeScrollRegion; top, bottom)
    }

    pub fn column_address(&mut self, x: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ColumnAddress; x)
    }

    pub fn cursor_address(&mut self, y: u32, x: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::CursorAddress; y, x)
    }

    pub fn erase_chars(&mut self, count: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::EraseChars; count)
    }

    pub fn parm_dch(&mut self, count: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ParmDch; count)
    }

    pub fn parm_delete_line(&mut self, count: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ParmDeleteLine; count)
    }

    pub fn parm_down_cursor(&mut self, count: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ParmDownCursor; count)
    }

    pub fn parm_ich(&mut self, count: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ParmIch; count)
    }

    pub fn parm_index(&mut self, count: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ParmIndex; count)
    }

    pub fn parm_insert_line(&mut self, count: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ParmInsertLine; count)
    }

    pub fn parm_left_cursor(&mut self, count: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ParmLeftCursor; count)
    }

    pub fn parm_right_cursor(&mut self, count: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ParmRightCursor; count)
    }
    pub fn parm_rindex(&mut self, count: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ParmRindex; count)
    }

    pub fn parm_up_cursor(&mut self, count: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ParmUpCursor; count)
    }

    pub fn parm_down_micro(&mut self, count: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ParmDownMicro; count)
    }

    pub fn parm_left_micro(&mut self, count: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ParmLeftMicro; count)
    }

    pub fn parm_right_micro(&mut self, count: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ParmRightMicro; count)
    }

    pub fn parm_up_micro(&mut self, count: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::ParmUpMicro; count)
    }

    pub fn row_address(&mut self, y: u32) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::RowAddress; y)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_attributes(
        &mut self,
        standout: bool,
        underline: bool,
        reverse: bool,
        blink: bool,
        dim: bool,
        bold: bool,
        invisible: bool,
        protected: bool,
        alt_charset: bool,
    ) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetAttributes; standout, underline, reverse, blink, dim, bold, invisible, protected, alt_charset)
    }

    pub fn set_a_foreground(&mut self, color: u8) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetAForeground; color)
    }

    pub fn set_a_background(&mut self, color: u8) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetABackground; color)
    }

    pub fn set_foreground(&mut self, color: u8) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetForeground; color)
    }

    pub fn set_background(&mut self, color: u8) -> Result<(), CapabilityError> {
        tty_expand_cap!(self.db, &mut self.buffer, cap::SetBackground; color)
    }

    // Some caps are still missing

    pub fn expand<C>(&'a mut self) -> Result<terminfo::Value, CapabilityError>
    where
        C: terminfo::Capability<'a> + AsRef<[u8]>,
    {
        todo!()
    }

    pub fn get_parser(&self) -> InputParser {
        InputParser::from_terminfo(&self.db)
    }
}

impl std::io::Write for TerminfoWrapper {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.buffer.flush()
    }
}

impl From<terminfo::Database> for TerminfoWrapper {
    fn from(value: terminfo::Database) -> Self {
        Self {
            db: value,
            buffer: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terminfo::Database;

    #[test]
    fn test() {
        let mut db =
            TerminfoWrapper::from(Database::from_path("assets/test_kitty_database").unwrap());
        let mut bytes = Vec::new();
        db.move_cursor(0, 0).unwrap();
        db.bell().unwrap();
        db.enter_bold_mode().unwrap();
        db.exit_attribute_mode().unwrap();
        db.flush_to(&mut bytes).unwrap();
        assert_eq!(
            b"\x1B[1;1H\
            \x07\
            \x1B[1m\
            \x1B(B\
            \x1B[m",
            &*bytes
        );
    }
}
