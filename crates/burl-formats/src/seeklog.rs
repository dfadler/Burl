//! `src/core/seeklog.cpp`: `SeekLog` (file, whole content in memory) and `StreamLog`
//! (stdin). The `SeekLog` models the `std::istream` state bits explicitly because callers
//! observe `isFinished()` timing, not just the lines.

use std::fmt;
use std::io::{BufRead, Read};

/// `gSeekLogMaxBufferSize`.
pub const SEEKLOG_MAX_BUFFER_SIZE: u64 = 104_857_600;

#[derive(Clone, Debug)]
pub struct SeekLogError(pub String);

impl fmt::Display for SeekLogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unable to read log file {}", self.0)
    }
}

impl std::error::Error for SeekLogError {}

fn strip_cr(line: &mut Vec<u8>) {
    if line.last() == Some(&b'\r') {
        line.pop();
    }
}

#[derive(Debug)]
pub struct SeekLog {
    buf: Vec<u8>,
    pos: usize,
    file_size: u64,
    eof: bool,
    fail: bool,
    current_percent: f32,
}

impl SeekLog {
    /// `readFully`: size from `tellg` at end, then the whole file. Files up to 100 MiB are
    /// copied through `std::string(const char*)`, which truncates at the first NUL byte.
    /// ponytail: larger files are also read whole here (C++ streams them from disk); the
    /// NUL truncation is skipped for them exactly as in C++.
    pub fn open(logfile: &str) -> Result<SeekLog, SeekLogError> {
        let mut buf = std::fs::read(logfile).map_err(|_| SeekLogError(logfile.to_string()))?;
        let file_size = buf.len() as u64;
        if file_size <= SEEKLOG_MAX_BUFFER_SIZE {
            if let Some(nul) = buf.iter().position(|&b| b == 0) {
                buf.truncate(nul);
            }
        }
        Ok(SeekLog { buf, pos: 0, file_size, eof: false, fail: false, current_percent: 0.0 })
    }

    pub fn from_bytes(buf: Vec<u8>) -> SeekLog {
        let file_size = buf.len() as u64;
        let mut buf = buf;
        if let Some(nul) = buf.iter().position(|&b| b == 0) {
            buf.truncate(nul);
        }
        SeekLog { buf, pos: 0, file_size, eof: false, fail: false, current_percent: 0.0 }
    }

    fn clear(&mut self) {
        self.eof = false;
        self.fail = false;
    }

    /// `getPercent`: percent of the last successful line (0 before the first read).
    pub fn get_percent(&self) -> f32 {
        self.current_percent
    }

    /// `setPointer`.
    pub fn set_pointer(&mut self, pointer: usize) {
        self.clear();
        self.pos = pointer.min(self.buf.len());
    }

    /// `seekTo(percent)`: f32 multiply, then discard the rest of the line unless at 0.
    pub fn seek_to(&mut self, percent: f32) {
        let mem_offset = (percent * self.file_size as f32) as usize;
        self.set_pointer(mem_offset);
        if mem_offset != 0 {
            let mut eol = Vec::new();
            self.next_line(&mut eol);
        }
    }

    /// `getNextLine`: `clear()` if finished, `std::getline`, strip one `\r`, false iff
    /// `fail()`. A final line without `\n` is returned with eofbit set, and the following
    /// `tellg()` then sets failbit and yields -1 (percent = -1/size), as libc++/libstdc++ do.
    pub fn next_line(&mut self, line: &mut Vec<u8>) -> bool {
        if self.is_finished() {
            self.clear();
        }
        line.clear();
        if self.pos >= self.buf.len() {
            self.eof = true;
            self.fail = true;
            return false;
        }
        let rest = &self.buf[self.pos..];
        match rest.iter().position(|&b| b == b'\n') {
            Some(i) => {
                line.extend_from_slice(&rest[..i]);
                self.pos += i + 1;
            }
            None => {
                line.extend_from_slice(rest);
                self.pos = self.buf.len();
                self.eof = true;
            }
        }
        strip_cr(line);
        if self.fail {
            return false;
        }
        if self.eof {
            self.fail = true;
            self.current_percent = -1.0 / self.file_size as f32;
        } else {
            self.current_percent = self.pos as f32 / self.file_size as f32;
        }
        true
    }

    /// `isFinished`: `fail() || eof()`.
    pub fn is_finished(&self) -> bool {
        self.fail || self.eof
    }
}

/// `StreamLog`: stdin. C++ puts stdin in non-blocking mode and treats "no data yet" like
/// EOF (retrying next call); it returns false for a line that is not `\n`-terminated, so an
/// unterminated last line is dropped, and so is the first half of a line split across pipe
/// writes. Blocking reads give the same results for any input delivered in full and do not
/// lose split lines; the partial-line loss is a C++ bug not reproduced.
#[derive(Debug, Default)]
pub struct StreamLog;

impl StreamLog {
    /// `std::cin.peek()`: first unread byte, `None` at EOF.
    pub fn peek() -> Option<u8> {
        std::io::stdin().lock().fill_buf().ok().and_then(|b| b.first().copied())
    }

    pub fn next_line(&mut self, line: &mut Vec<u8>) -> bool {
        line.clear();
        let mut stdin = std::io::stdin().lock();
        match stdin.read_until(b'\n', line) {
            Ok(0) | Err(_) => false,
            Ok(_) => {
                if line.last() != Some(&b'\n') {
                    line.clear();
                    return false;
                }
                line.pop();
                strip_cr(line);
                true
            }
        }
    }
}

/// First byte of a file via `std::ifstream::peek()`; `None` if unreadable or empty.
pub fn peek_file(path: &str) -> Option<u8> {
    let mut f = std::fs::File::open(path).ok()?;
    let mut b = [0u8; 1];
    match f.read(&mut b) {
        Ok(1) => Some(b[0]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(log: &mut SeekLog) -> (Vec<Vec<u8>>, Vec<bool>) {
        let mut out = Vec::new();
        let mut finished = Vec::new();
        let mut l = Vec::new();
        while log.next_line(&mut l) {
            out.push(l.clone());
            finished.push(log.is_finished());
        }
        (out, finished)
    }

    #[test]
    fn crlf_and_terminated_last_line() {
        let mut log = SeekLog::from_bytes(b"a\r\nb\n".to_vec());
        let (out, fin) = lines(&mut log);
        assert_eq!(out, vec![b"a".to_vec(), b"b".to_vec()]);
        // a file ending in \n is not finished until a read fails
        assert_eq!(fin, vec![false, false]);
        assert!(log.is_finished());
        assert_eq!(log.get_percent(), 1.0);
    }

    #[test]
    fn unterminated_last_line_is_returned_then_finished() {
        let mut log = SeekLog::from_bytes(b"a\nb".to_vec());
        let (out, fin) = lines(&mut log);
        assert_eq!(out, vec![b"a".to_vec(), b"b".to_vec()]);
        assert_eq!(fin, vec![false, true]);
        assert_eq!(log.get_percent(), -1.0 / 3.0);
    }

    #[test]
    fn nul_truncates_and_empty_file() {
        let mut log = SeekLog::from_bytes(b"a\n\0b\n".to_vec());
        let (out, _) = lines(&mut log);
        assert_eq!(out, vec![b"a".to_vec()]);
        let mut log = SeekLog::from_bytes(Vec::new());
        let mut l = Vec::new();
        assert!(!log.next_line(&mut l));
        assert!(log.is_finished());
    }

    #[test]
    fn seek_to_discards_partial_line() {
        let mut log = SeekLog::from_bytes(b"aaaa\nbbbb\ncccc\n".to_vec());
        log.seek_to(0.5);
        let mut l = Vec::new();
        assert!(log.next_line(&mut l));
        assert_eq!(l, b"cccc");
        log.seek_to(0.0);
        assert!(log.next_line(&mut l));
        assert_eq!(l, b"aaaa");
    }
}
