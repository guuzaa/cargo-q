//! Bounded capture of a command's merged stdout+stderr.

use super::append_stream;

/// Keep this much of each command's captured output (the tail). Cargo errors
/// land at the end; unbounded buffering of `cargo test` / `build -vv` would
/// otherwise grow without limit.
///
/// The live buffer may reach 2× this before the head is dropped, so each
/// retained byte is copied O(1) times rather than once per incoming chunk.
const OUTPUT_TAIL: usize = 128 * 1024;

const TRUNCATED_MARK: &[u8] = b"[... output truncated ...]\n";

/// Bounded capture of a command's merged stdout+stderr.
pub(crate) struct Tail {
    data: Vec<u8>,
    truncated: bool,
    cap: usize,
}

impl Tail {
    pub(crate) fn new() -> Self {
        Self::with_cap(OUTPUT_TAIL)
    }

    fn with_cap(cap: usize) -> Self {
        Self {
            data: Vec::new(),
            truncated: false,
            cap,
        }
    }

    pub(crate) fn push(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        self.data.extend_from_slice(bytes);
        if self.data.len() >= self.cap.saturating_mul(2) {
            self.compact_to_cap();
        }
    }

    fn compact_to_cap(&mut self) {
        if self.data.len() <= self.cap {
            return;
        }
        self.truncated = true;
        let excess = self.data.len() - self.cap;
        self.data.copy_within(excess.., 0);
        self.data.truncate(self.cap);
        if self.data.capacity() > self.cap.saturating_mul(2) {
            self.data.shrink_to(self.cap);
        }
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::with_cap(self.cap);
    }

    pub(crate) fn take(&mut self) -> Self {
        std::mem::replace(self, Self::with_cap(self.cap))
    }
}

/// Append a captured tail to `dst`, with a marker if the head was dropped.
pub(crate) fn write_tail(dst: &mut Vec<u8>, tail: &Tail) {
    if tail.truncated {
        dst.extend_from_slice(TRUNCATED_MARK);
    }
    append_stream(dst, &tail.data);
}

#[cfg(test)]
mod tests {
    #[test]
    fn output_tail_keeps_bytes_under_the_cap() {
        let mut tail = super::Tail::with_cap(16);
        tail.push(b"hello ");
        tail.push(b"world");
        assert!(!tail.truncated);
        assert_eq!(tail.data, b"hello world");
    }

    #[test]
    fn output_tail_keeps_everything_until_twice_cap() {
        let mut tail = super::Tail::with_cap(8);
        tail.push(b"abcdefghijkl");
        assert!(!tail.truncated);
        assert_eq!(tail.data, b"abcdefghijkl");
    }

    #[test]
    fn output_tail_keeps_the_end_when_over_twice_cap() {
        let mut tail = super::Tail::with_cap(8);
        tail.push(b"abcdefghijklmnop");
        assert!(tail.truncated);
        assert_eq!(tail.data, b"ijklmnop");

        // Slack until the next compact: the extra bytes stay, they are the tail.
        tail.push(b"XYZ");
        assert_eq!(tail.data, b"ijklmnopXYZ");
    }

    #[test]
    fn output_tail_keeps_the_suffix_across_many_chunks() {
        let mut tail = super::Tail::with_cap(8);
        for byte in b"abcdefghijklmnopqrstuvwxyz" {
            tail.push(std::slice::from_ref(byte));
        }
        assert!(tail.truncated);
        assert_eq!(tail.data, b"qrstuvwxyz");
    }

    #[test]
    fn write_tail_marks_truncated_output() {
        let mut tail = super::Tail::with_cap(4);
        tail.push(b"abcdefgh");
        let mut buf = Vec::new();
        super::write_tail(&mut buf, &tail);
        assert_eq!(buf, b"[... output truncated ...]\nefgh\n");
    }
}
