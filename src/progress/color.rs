use std::fmt;
use std::io::{self, IsTerminal};
use std::sync::OnceLock;

pub trait Colored {
    fn red(self) -> ColoredString;
    fn green(self) -> ColoredString;
    fn yellow(self) -> ColoredString;
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
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| io::stdout().is_terminal())
}

impl<T: fmt::Display> Colored for T {
    fn red(self) -> ColoredString {
        if color_enabled() {
            ColoredString(format!("\x1b[31m{self}\x1b[0m"))
        } else {
            ColoredString(self.to_string())
        }
    }
    fn green(self) -> ColoredString {
        if color_enabled() {
            ColoredString(format!("\x1b[32m{self}\x1b[0m"))
        } else {
            ColoredString(self.to_string())
        }
    }
    fn yellow(self) -> ColoredString {
        if color_enabled() {
            ColoredString(format!("\x1b[33m{self}\x1b[0m"))
        } else {
            ColoredString(self.to_string())
        }
    }
    fn bold(self) -> ColoredString {
        if color_enabled() {
            ColoredString(format!("\x1b[1m{self}\x1b[0m"))
        } else {
            ColoredString(self.to_string())
        }
    }
}
