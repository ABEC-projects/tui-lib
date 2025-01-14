use std::io::Write;

use super::Result;
use terminfo::capability as cap;

/// Used to configurate Tty in revertable way so that terminal will
/// be automatically restored to it's original state upon dropping Tty.
pub trait TtyChange {
    /// Applies the change
    fn apply(&self, tty: &mut std::fs::File) -> Result<()>;
    /// Reverts the change
    fn revert(&self, tty: &mut std::fs::File) -> Result<()>;
}

pub trait FromTerminfo: Sized {
    fn from_terminfo(db: &terminfo::Database) -> Option<Self>;
}

macro_rules! change_from_terminfo {
    ($name: ident, $apply: path, $restore: path) => {
        pub struct $name {
            apply: Vec<u8>,
            restore: Vec<u8>,
        }

        impl TtyChange for $name {
            fn apply(&self, tty: &mut std::fs::File) -> Result<()> {
                Ok(tty.write_all(&self.apply)?)
            }
            fn revert(&self, tty: &mut std::fs::File) -> Result<()> {
                Ok(tty.write_all(&self.restore)?)
            }
        }

        impl FromTerminfo for $name {
            fn from_terminfo(db: &::terminfo::Database) -> Option<Self> {
                Some(Self {
                    apply: db.get::<$apply>()?.as_ref().to_owned(),
                    restore: db.get::<$restore>()?.as_ref().to_owned(),
                })
            }
        }
    };
}

change_from_terminfo!(SaveCursor, cap::SaveCursor, cap::RestoreCursor);
change_from_terminfo!(EnterCaMode, cap::EnterCaMode, cap::ExitCaMode);
