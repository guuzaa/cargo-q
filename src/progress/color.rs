use std::fmt;
use std::io::{self, IsTerminal};

pub trait ColorExt {
    fn red(self) -> ColoredString;
    fn green(self) -> ColoredString;
    fn bold(self) -> ColoredString;
}

pub(crate) struct ColoredString(String);

impl fmt::Display for ColoredString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[inline]
fn color_enabled() -> bool {
    io::stdout().is_terminal()
}

impl<T: fmt::Display> ColorExt for T {
    fn red(self) -> ColoredString {
        if color_enabled() {
            ColoredString(format!("\x1b[31m{}\x1b[0m", self))
        } else {
            ColoredString(self.to_string())
        }
    }
    fn green(self) -> ColoredString {
        if color_enabled() {
            ColoredString(format!("\x1b[32m{}\x1b[0m", self))
        } else {
            ColoredString(self.to_string())
        }
    }
    fn bold(self) -> ColoredString {
        if color_enabled() {
            ColoredString(format!("\x1b[1m{}\x1b[0m", self))
        } else {
            ColoredString(self.to_string())
        }
    }
}
