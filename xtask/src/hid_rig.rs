//! Real-device relative mouse for `cargo xtask display-harness --grab-device` (ticket #211).
//!
//! Hosted runners cannot prove that a real device's relative motion is detached from the cursor
//! under a macOS `Locked` grab (#205). Two routes reach the real HID path, both behind
//! [`RelativeMouse`] and one line protocol (see `tools/hid-rig/README.md`):
//! - **serial rig**: a Pico acting as a USB HID mouse, driven over a serial port (any machine);
//! - **Karabiner helper**: a root helper built on Karabiner-DriverKit-VirtualHIDDevice, run through
//!   `BURL_VHID_HELPER` (a local Mac only: one-time system-extension approval, root daemon).
//!
//! Everything here is std only. The protocol and the verdict are pure and unit-tested; the device
//! I/O is UNRUN until the rig exists.

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The largest `rel` component: one signed HID byte, what the firmware's `Mouse.move` takes.
pub const MAX_STEP: i32 = 127;
/// Env var naming the Karabiner helper executable (speaks the same protocol on stdin/stdout).
pub const HELPER_ENV: &str = "BURL_VHID_HELPER";

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Cmd {
    Ping,
    Rel(i32, i32),
    Down,
    Up,
}

/// The wire form of `c`, newline included. `Rel` outside `-127..=127` is an error, not a clamp:
/// a silently shortened move would make the cursor-delta verdict lie.
pub fn encode(c: Cmd) -> Result<String, String> {
    Ok(match c {
        Cmd::Ping => "ping\n".into(),
        Cmd::Down => "down\n".into(),
        Cmd::Up => "up\n".into(),
        Cmd::Rel(dx, dy) if dx.abs() <= MAX_STEP && dy.abs() <= MAX_STEP => {
            format!("rel {dx} {dy}\n")
        }
        Cmd::Rel(dx, dy) => {
            return Err(format!(
                "rel {dx} {dy}: each part must be within +-{MAX_STEP}"
            ))
        }
    })
}

/// The inverse of [`encode`], as the firmware reads a line (trailing CR/LF ignored).
pub fn parse(line: &str) -> Result<Cmd, String> {
    let mut t = line.trim_end_matches(['\r', '\n']).split(' ');
    let bad = || format!("bad command {line:?}");
    let num = |t: Option<&str>| {
        t.and_then(|s| s.parse::<i32>().ok())
            .filter(|n| n.abs() <= MAX_STEP)
    };
    let c = match t.next() {
        Some("ping") => Cmd::Ping,
        Some("down") => Cmd::Down,
        Some("up") => Cmd::Up,
        Some("rel") => {
            let (dx, dy) = (num(t.next()), num(t.next()));
            match (dx, dy) {
                (Some(dx), Some(dy)) => Cmd::Rel(dx, dy),
                _ => return Err(bad()),
            }
        }
        _ => return Err(bad()),
    };
    if t.next().is_some() {
        return Err(bad());
    }
    Ok(c)
}

/// The firmware's answer to one command: `ok [info]` gives `Ok(info)`, `err MSG` gives `Err(MSG)`.
pub fn parse_reply(line: &str) -> Result<String, String> {
    let l = line.trim();
    match l.split_once(' ').unwrap_or((l, "")) {
        ("ok", info) => Ok(info.to_string()),
        ("err", msg) => Err(format!("device refused: {msg}")),
        _ => Err(format!("unexpected reply {l:?}")),
    }
}

/// A device that produces RELATIVE motion and a left button, through the real HID path.
pub trait RelativeMouse {
    fn name(&self) -> String;
    /// Is the device there and answering? `Err` is the reason it is not.
    fn available(&mut self) -> Result<(), String>;
    fn rel(&mut self, dx: i32, dy: i32) -> Result<(), String>;
    fn down(&mut self) -> Result<(), String>;
    fn up(&mut self) -> Result<(), String>;
}

/// The line protocol over any byte pipe (a serial file, a helper's stdin/stdout).
pub struct Line<R: Read, W: Write> {
    name: String,
    rx: R,
    tx: W,
    child: Option<Child>,
    /// One deadline for a whole command exchange (the write and every byte of the reply): `send`
    /// moves the shared instant to now + the window, and the reader gives up when it passes. `None`
    /// for the serial port, whose `stty` read timeout does that job.
    deadline: Option<(Arc<Mutex<Instant>>, Duration)>,
}

impl<R: Read, W: Write> Drop for Line<R, W> {
    fn drop(&mut self) {
        if let Some(c) = &mut self.child {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

/// One reply line. A read of 0 bytes is the serial port's `time` timeout (see [`serial`]) or a
/// helper that exited.
fn read_line(rx: &mut impl Read) -> Result<String, String> {
    let mut line = Vec::new();
    let mut b = [0u8; 1];
    while line.len() < 200 {
        match rx.read(&mut b) {
            Ok(0) => {
                return Err(format!(
                    "no reply (timeout or closed) after {:?}",
                    String::from_utf8_lossy(&line)
                ))
            }
            Ok(_) if b[0] == b'\n' => return Ok(String::from_utf8_lossy(&line).into_owned()),
            Ok(_) => line.push(b[0]),
            Err(e) => return Err(format!("read: {e}")),
        }
    }
    Err("reply line too long".into())
}

impl<R: Read, W: Write> Line<R, W> {
    fn send(&mut self, c: Cmd) -> Result<String, String> {
        if let Some((at, window)) = &self.deadline {
            *at.lock().unwrap() = Instant::now() + *window;
        }
        // A command is a few bytes and every one is answered before the next is sent, so the helper's
        // stdin pipe (64 KiB) cannot fill: this write cannot block on a helper that stopped reading,
        // the missing reply is what times out (and the failed backend is dropped, which kills it).
        self.tx
            .write_all(encode(c)?.as_bytes())
            .and_then(|()| self.tx.flush())
            .map_err(|e| format!("{}: write: {e}", self.name))?;
        parse_reply(&read_line(&mut self.rx).map_err(|e| format!("{}: {e}", self.name))?)
    }
}

impl<R: Read, W: Write> RelativeMouse for Line<R, W> {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn available(&mut self) -> Result<(), String> {
        let id = self.send(Cmd::Ping)?;
        if id.starts_with("burl-hid-rig") {
            Ok(())
        } else {
            Err(format!(
                "{}: answered ping with {id:?}, not `burl-hid-rig ..`",
                self.name
            ))
        }
    }
    fn rel(&mut self, dx: i32, dy: i32) -> Result<(), String> {
        self.send(Cmd::Rel(dx, dy)).map(|_| ())
    }
    fn down(&mut self) -> Result<(), String> {
        self.send(Cmd::Down).map(|_| ())
    }
    fn up(&mut self) -> Result<(), String> {
        self.send(Cmd::Up).map(|_| ())
    }
}

/// The serial backend: `/dev/cu.*` opened raw with a 2 s read timeout (`stty min 0 time 20`).
/// The port is opened BEFORE `stty` runs because macOS may reset a tty's settings on last close.
pub fn serial(path: &str) -> Result<Box<dyn RelativeMouse>, String> {
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| format!("{path}: {e}"))?;
    let ok = Command::new("stty")
        .args([
            "-f", path, "115200", "raw", "-echo", "min", "0", "time", "20",
        ])
        .status();
    if !ok.is_ok_and(|s| s.success()) {
        return Err(format!("{path}: `stty -f` failed"));
    }
    let tx = f.try_clone().map_err(|e| format!("{path}: {e}"))?;
    Ok(Box::new(Line {
        name: format!("serial {path}"),
        rx: f,
        tx,
        child: None,
        deadline: None,
    }))
}

/// The Karabiner backend: runs the helper named by `BURL_VHID_HELPER`, which must speak the line
/// protocol on stdin/stdout and run as root (the virtual devices accept root clients only).
pub fn helper(path: &str) -> Result<Box<dyn RelativeMouse>, String> {
    helper_cmd(Command::new(path), path, HELPER_REPLY_TIMEOUT)
}

/// How long to wait for one helper reply before giving up on it.
const HELPER_REPLY_TIMEOUT: Duration = Duration::from_secs(2);

/// A helper's stdout read against the exchange's shared deadline. A pipe from a live but silent
/// process never reaches EOF, so a plain `read` would hang forever, and a per-read timeout lets a
/// trickle of bytes stretch one reply without end; this reads on a thread and returns `Ok(0)` (the
/// same "no reply" the serial port's `time` timeout gives) once the deadline passes. When the
/// `Line` is dropped its child is killed and reaped, which also ends the thread.
struct TimedReader {
    rx: Receiver<Vec<u8>>,
    pending: Vec<u8>,
    deadline: Arc<Mutex<Instant>>,
}

impl Read for TimedReader {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if self.pending.is_empty() {
            let left = self
                .deadline
                .lock()
                .unwrap()
                .saturating_duration_since(Instant::now());
            match self.rx.recv_timeout(left) {
                Ok(chunk) => self.pending = chunk,
                Err(_) => return Ok(0), // timed out, or the helper closed its stdout
            }
        }
        let n = out.len().min(self.pending.len());
        out[..n].copy_from_slice(&self.pending[..n]);
        self.pending.drain(..n);
        Ok(n)
    }
}

fn helper_cmd(
    mut cmd: Command,
    label: &str,
    timeout: Duration,
) -> Result<Box<dyn RelativeMouse>, String> {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{label}: {e}"))?;
    let (tx, mut stdout) = (
        child.stdin.take().ok_or("no stdin")?,
        child.stdout.take().ok_or("no stdout")?,
    );
    let (chunks, rx) = channel();
    std::thread::spawn(move || {
        let mut buf = [0u8; 256];
        while let Ok(n) = stdout.read(&mut buf) {
            if n == 0 || chunks.send(buf[..n].to_vec()).is_err() {
                break;
            }
        }
    });
    let at = Arc::new(Mutex::new(Instant::now()));
    Ok(Box::new(Line {
        name: format!("karabiner helper {label}"),
        rx: TimedReader {
            rx,
            pending: Vec::new(),
            deadline: Arc::clone(&at),
        },
        tx,
        child: Some(child),
        deadline: Some((at, timeout)),
    }))
}

/// Candidate rig ports on macOS: the Pico's CDC serial shows up as `/dev/cu.usbmodem*`.
pub fn detect_ports() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir("/dev")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with("cu.usbmodem"))
        .map(|n| format!("/dev/{n}"))
        .collect();
    v.sort();
    v
}

/// `--grab-device [--port PATH]`: the port, if given.
pub fn parse_args(args: &[String]) -> Result<Option<String>, String> {
    match args {
        [f] if f == "--grab-device" => Ok(None),
        [f, p, v] if f == "--grab-device" && p == "--port" && !v.starts_with('-') => {
            Ok(Some(v.clone()))
        }
        _ => Err("usage: cargo xtask display-harness --grab-device [--port PATH]".into()),
    }
}

/// What `--grab-device` runs on: `Ok(Some)` a device that answered, `Ok(None)` none to be found
/// (SKIP), `Err` a device the user pointed at that does not work (a failure, never a skip).
pub fn pick_backend(port: Option<&str>) -> Result<Option<Box<dyn RelativeMouse>>, String> {
    let try_one = |mut b: Box<dyn RelativeMouse>| b.available().map(|()| b);
    if let Some(p) = port {
        return serial(p).and_then(try_one).map(Some);
    }
    if let Ok(h) = std::env::var(HELPER_ENV) {
        return helper(&h).and_then(try_one).map(Some);
    }
    // Autodetect: the first port that answers wins; ports that do not (other USB modems) are not errors.
    Ok(detect_ports()
        .iter()
        .find_map(|p| serial(p).and_then(try_one).ok()))
}

/// Cursor must move at least this many points in the control (the device reaches the system).
pub const MIN_CONTROL_PT: i32 = 20;
/// Under the lock the cursor may drift at most this many points (rounding, a stray sub-pixel).
pub const LOCK_TOLERANCE_PT: i32 = 3;

/// The verdict. `control`, `locked` and `released` are the cursor's displacement in points over
/// each phase (no grab; button held; button up again); `sent` is the device counts sent in each.
/// The control must move the cursor with the right sign (acceleration scales the distance, so only
/// a floor is checked); only then does a still cursor under the lock mean anything. After the
/// release the cursor must follow the device again by the same floor: a log line saying the grab
/// is off does not prove the cursor was freed.
pub fn check_device_motion(
    control: (i32, i32),
    locked: (i32, i32),
    released: (i32, i32),
    sent: (i32, i32),
) -> Result<(), String> {
    let along = |d: (i32, i32)| d.0 * sent.0.signum() + d.1 * sent.1.signum();
    if along(control) < MIN_CONTROL_PT {
        return Err(format!(
            "control failed: the device sent {sent:?} counts with no grab and the cursor moved {control:?} pt; \
             the device does not reach the system (Input Monitoring / pointer settings?), so the lock proves nothing"
        ));
    }
    let drift = locked.0.abs().max(locked.1.abs());
    if drift > LOCK_TOLERANCE_PT {
        return Err(format!(
            "the cursor moved {locked:?} pt under the Locked grab (tolerance {LOCK_TOLERANCE_PT}) while the device sent {sent:?} counts"
        ));
    }
    if along(released) < MIN_CONTROL_PT {
        return Err(format!(
            "the cursor stayed detached after the release: the device sent {sent:?} counts with the button up and the cursor moved {released:?} pt (floor {MIN_CONTROL_PT})"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_and_parse_agree_on_every_command() {
        for c in [
            Cmd::Ping,
            Cmd::Down,
            Cmd::Up,
            Cmd::Rel(5, -7),
            Cmd::Rel(-127, 127),
            Cmd::Rel(0, 0),
        ] {
            assert_eq!(parse(&encode(c).unwrap()), Ok(c));
        }
        assert_eq!(encode(Cmd::Rel(5, -7)).unwrap(), "rel 5 -7\n");
    }

    #[test]
    fn out_of_range_and_malformed_lines_are_refused() {
        assert!(encode(Cmd::Rel(128, 0)).is_err());
        assert!(encode(Cmd::Rel(0, -128)).is_err());
        for l in [
            "rel 1",
            "rel 1 2 3",
            "rel a b",
            "rel 200 0",
            "down now",
            "jump",
            "",
            "rel  1 2",
        ] {
            assert!(parse(l).is_err(), "{l:?}");
        }
    }

    #[test]
    fn replies_split_into_ok_and_err() {
        assert_eq!(
            parse_reply("ok burl-hid-rig 1\r"),
            Ok("burl-hid-rig 1".into())
        );
        assert_eq!(parse_reply("ok"), Ok(String::new()));
        assert!(parse_reply("err bad range")
            .unwrap_err()
            .contains("bad range"));
        assert!(parse_reply("garbage").is_err());
    }

    fn line(reply: &str) -> Line<&[u8], Vec<u8>> {
        Line {
            name: "t".into(),
            rx: reply.as_bytes(),
            tx: Vec::new(),
            child: None,
            deadline: None,
        }
    }

    #[test]
    fn the_line_backend_writes_the_wire_form_and_reads_replies() {
        let mut l = line("ok burl-hid-rig 1\nok\nok\nok\n");
        l.available().unwrap();
        l.rel(3, -4).unwrap();
        l.down().unwrap();
        l.up().unwrap();
        assert_eq!(
            String::from_utf8(l.tx.clone()).unwrap(),
            "ping\nrel 3 -4\ndown\nup\n"
        );
    }

    #[test]
    fn silence_a_refusal_and_a_stranger_are_errors() {
        assert!(line("").rel(1, 1).unwrap_err().contains("no reply"));
        assert!(line("err no\n").down().unwrap_err().contains("refused"));
        assert!(line("ok hello\n").available().is_err());
    }

    #[test]
    fn argument_parsing() {
        let a = |v: &[&str]| parse_args(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(a(&["--grab-device"]), Ok(None));
        assert_eq!(
            a(&["--grab-device", "--port", "/dev/cu.usbmodem1"]),
            Ok(Some("/dev/cu.usbmodem1".into()))
        );
        assert!(a(&["--grab-device", "--port"]).is_err());
        assert!(a(&["--grab-device", "--bogus"]).is_err());
    }

    #[test]
    fn a_free_cursor_that_moves_with_a_still_one_under_the_lock_passes() {
        assert_eq!(
            check_device_motion((60, 0), (0, 0), (50, 0), (100, 0)),
            Ok(())
        );
        assert_eq!(
            check_device_motion((-60, 1), (-2, 3), (-40, 0), (-100, 0)),
            Ok(())
        );
    }

    #[test]
    fn a_cursor_that_follows_the_device_under_the_lock_fails() {
        let e = check_device_motion((60, 0), (55, 0), (50, 0), (100, 0)).unwrap_err();
        assert!(e.contains("under the Locked grab"), "{e}");
        assert!(check_device_motion((60, 0), (0, -4), (50, 0), (100, 0)).is_err());
    }

    #[test]
    fn a_control_that_does_not_move_proves_nothing() {
        let e = check_device_motion((2, 0), (0, 0), (50, 0), (100, 0)).unwrap_err();
        assert!(e.contains("control failed"), "{e}");
        // wrong direction is no control either
        assert!(check_device_motion((-60, 0), (0, 0), (50, 0), (100, 0)).is_err());
    }

    #[test]
    fn a_silent_helper_times_out_instead_of_hanging() {
        // `sleep` stays alive and never writes: a plain pipe read would block for the full 30 s.
        let mut cmd = Command::new("sleep");
        cmd.arg("30");
        let mut dev = helper_cmd(cmd, "sleep", Duration::from_millis(150)).unwrap();
        let t = std::time::Instant::now();
        let e = dev.available().unwrap_err();
        assert!(e.contains("no reply"), "{e}");
        assert!(t.elapsed() < Duration::from_secs(5), "{:?}", t.elapsed());
    }

    #[test]
    fn a_helper_that_trickles_bytes_still_hits_the_exchange_deadline() {
        // One byte every 100 ms and never a newline: a per-read timeout of 400 ms would be reset by
        // every byte and the 200-byte reply cap would take ~20 s; one deadline per exchange stops it.
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "while true; do printf x; sleep 0.1; done"]);
        let mut dev = helper_cmd(cmd, "trickle", Duration::from_millis(400)).unwrap();
        let t = Instant::now();
        let e = dev.available().unwrap_err();
        assert!(e.contains("no reply"), "{e}");
        assert!(t.elapsed() < Duration::from_secs(3), "{:?}", t.elapsed());
    }

    #[test]
    fn a_cursor_that_stays_detached_after_the_release_fails() {
        let e = check_device_motion((60, 0), (0, 0), (1, 0), (100, 0)).unwrap_err();
        assert!(e.contains("stayed detached after the release"), "{e}");
        // moving the wrong way after the release is no proof either
        assert!(check_device_motion((60, 0), (0, 0), (-50, 0), (100, 0)).is_err());
    }
}
