//! `cargo xtask cursor-lab`: a measurement LAB, not a check. It runs burl as `--grab` does and
//! measures, on a hosted macOS runner, what the cursor does under a held grab for several ways of
//! injecting motion, whether a listen-only event tap sees the deltas, whether the cursor sprite is
//! hidden in screen captures, and whether a virtual HID mouse can be created. It prints a table and
//! fails only on infrastructure errors (a helper that will not build, no window, a helper that
//! cannot post). See "Cursor lab (experiment)" in `tools/display-harness/README.md`.

use std::cell::{Cell, RefCell};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use crate::display_harness::{parse_grab_events, parse_window, Window};
use crate::ssim::Image;

// ---- pure parsing and decisions (unit-tested below) ----

/// One line of `tapwatch`: `event type=moved x=412.0 y=390.0 dx=10 dy=0`.
#[derive(Debug, Clone, PartialEq)]
pub struct TapEvent {
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub dx: i64,
    pub dy: i64,
}

pub fn parse_tap_line(line: &str) -> Option<TapEvent> {
    let rest = line.strip_prefix("event ")?;
    let f = |k: &str| rest.split_whitespace().find_map(|t| t.strip_prefix(k));
    Some(TapEvent {
        kind: f("type=")?.to_string(),
        x: f("x=")?.parse().ok()?,
        y: f("y=")?.parse().ok()?,
        dx: f("dx=")?.parse().ok()?,
        dy: f("dy=")?.parse().ok()?,
    })
}

pub fn parse_tap_events(text: &str) -> Vec<TapEvent> {
    text.lines().filter_map(parse_tap_line).collect()
}

/// What the observer saw of the moved/dragged events (button events are not counted).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TapSummary {
    pub count: usize,
    pub dx: i64,
    pub dy: i64,
    /// Location of the first and last counted event.
    pub first: Option<(f64, f64)>,
    pub last: Option<(f64, f64)>,
}

pub fn summarize_tap(events: &[TapEvent]) -> TapSummary {
    let mut s = TapSummary::default();
    for e in events
        .iter()
        .filter(|e| e.kind == "moved" || e.kind == "dragged")
    {
        s.count += 1;
        s.dx += e.dx;
        s.dy += e.dy;
        s.first.get_or_insert((e.x, e.y));
        s.last = Some((e.x, e.y));
    }
    s
}

/// `tapwatch`'s first line: `access trusted=1 listen=0` -> (AXIsProcessTrusted, listen access).
pub fn parse_access(text: &str) -> Option<(bool, bool)> {
    let line = text.lines().find(|l| l.starts_with("access "))?;
    let f = |k: &str| line.split_whitespace().find_map(|t| t.strip_prefix(k));
    Some((f("trusted=")? == "1", f("listen=")? == "1"))
}

/// `mouse pos` / `mouse bounds` output: two integers.
pub fn parse_xy(out: &str) -> Option<(i32, i32)> {
    let mut it = out.split_whitespace().map(|t| t.parse::<i32>());
    match (it.next(), it.next()) {
        (Some(Ok(x)), Some(Ok(y))) => Some((x, y)),
        _ => None,
    }
}

/// `mouse vis` output: `visible=1` or `visible=0`.
pub fn parse_visible(out: &str) -> Option<bool> {
    match out.trim().strip_prefix("visible=")? {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }
}

/// `exit code N` or `killed by signal N`: what a helper that did not finish normally reports.
pub fn describe_exit(st: &ExitStatus) -> String {
    use std::os::unix::process::ExitStatusExt;
    match (st.code(), st.signal()) {
        (Some(c), _) => format!("exit code {c}"),
        (_, Some(s)) => format!("killed by signal {s}"),
        _ => st.to_string(),
    }
}

/// One way of injecting motion with `mouse`.
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub name: &'static str,
    /// Relative-only (`mouse rel`) rather than an absolute location.
    pub rel: bool,
    pub tap: &'static str,
    pub source_hid: bool,
}

/// V1 (control) and V2/V3: absolute, relative-only, and the other tap locations and source.
pub fn variants() -> Vec<Variant> {
    let v = |name, rel, tap, source_hid| Variant {
        name,
        rel,
        tap,
        source_hid,
    };
    vec![
        v("V1 abs @hid", false, "hid", false),
        v("V2 rel @hid", true, "hid", false),
        v("V3 rel @session", true, "session", false),
        v("V3 rel @annotated", true, "annotated", false),
        v("V3 rel @hid +HIDsrc", true, "hid", true),
        v("V3 rel @session +HIDsrc", true, "session", true),
        v("V3 abs @session +HIDsrc", false, "session", true),
    ]
}

/// Points per event and events per variant: 5 x 10 = 50 points, as the `--grab` measurement.
pub const STEP: i32 = 10;
pub const EVENTS: i32 = 5;

/// The `mouse` argument list for event `k` (1-based) of a variant. `kind` is `moved` or `dragged`;
/// `base` is the cursor position when the variant started; `sign` is +1 or -1 (variants alternate so
/// the cursor does not walk off the window).
pub fn variant_args(v: &Variant, kind: &str, base: (i32, i32), k: i32, sign: i32) -> Vec<String> {
    let d = sign * STEP;
    let mut a: Vec<String> = if v.rel {
        vec![
            "rel".into(),
            d.to_string(),
            "0".into(),
            "--type".into(),
            kind.into(),
        ]
    } else {
        let cmd = if kind == "dragged" { "drag" } else { "move" };
        vec![
            cmd.into(),
            (base.0 + k * d).to_string(),
            base.1.to_string(),
            d.to_string(),
            "0".into(),
        ]
    };
    a.extend(["--tap".into(), v.tap.into()]);
    if v.source_hid {
        a.extend(["--source".into(), "hid".into()]);
    }
    a
}

/// What the cursor did to the posted motion, for one variant, comparing no grab with a held grab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// The cursor did not follow the posted motion even with no grab: the variant tells nothing.
    Uninformative,
    /// It followed with no grab and stayed (nearly) still under the grab: an assertable lock.
    Detached,
    /// It followed with no grab and also under the grab: the lock is invisible to this variant.
    IgnoresLock,
}

/// `none` / `grab`: net |dx| of the cursor with no grab / under the grab; `posted`: points posted.
pub fn classify(none: i32, grab: i32, posted: i32) -> Effect {
    if none.abs() < posted / 2 {
        Effect::Uninformative
    } else if grab.abs() <= posted / 10 {
        Effect::Detached
    } else {
        Effect::IgnoresLock
    }
}

/// One measured cell of the table.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub phase: String,
    pub variant: String,
    pub before: (i32, i32),
    pub after: (i32, i32),
    pub visible: Option<bool>,
    pub tap: Option<TapSummary>,
    /// For a held-grab phase: burl's `grab: event=on` line was seen with `mode=locked`. A row
    /// measured without a confirmed locked grab (no `on` line, `mode=none` meaning both grab
    /// requests failed, or `confined`) must not yield a lock verdict. Always true outside a grab.
    pub locked: bool,
}

/// Phases measured while burl holds a grab: `grab/..` and `<label>/grab` (not `no-grab/..`).
pub fn is_grab_phase(phase: &str) -> bool {
    phase.starts_with("grab/") || phase.ends_with("/grab")
}

pub fn header() -> String {
    format!(
        "{:<14} {:<26} {:<12} {:<12} {:<9} {:<8} {}",
        "phase",
        "variant",
        "before",
        "after",
        "moved dx",
        "visible",
        "tap (n, sum dx,dy, first->last x)"
    )
}

pub fn format_row(r: &Row) -> String {
    let tap = r.tap.as_ref().map_or("tap unavailable".to_string(), |t| {
        let at = |p: Option<(f64, f64)>| p.map_or("-".to_string(), |p| format!("{:.0}", p.0));
        format!(
            "{}, {},{}, {}->{}",
            t.count,
            t.dx,
            t.dy,
            at(t.first),
            at(t.last)
        )
    });
    let vis = r
        .visible
        .map_or("?".to_string(), |v| (if v { "1" } else { "0" }).to_string());
    format!(
        "{:<14} {:<26} {:<12} {:<12} {:<9} {:<8} {}",
        r.phase,
        r.variant,
        format!("{:?}", r.before),
        format!("{:?}", r.after),
        r.after.0 - r.before.0,
        vis,
        tap
    )
}

/// The log line for a row whose injection failed: the measurement is invalid, not "cursor held".
pub fn format_invalid_row(r: &Row, why: &str) -> String {
    format!(
        "{}  INVALID: injection failed ({why}); left out of the table and the verdicts",
        format_row(r)
    )
}

/// Per variant: the verdict from the `no-grab/moved` row against each held-grab row.
pub fn summary(rows: &[Row]) -> Vec<String> {
    let posted = STEP * EVENTS;
    let dx = |phase: &str, name: &str| {
        rows.iter()
            .find(|r| r.phase == phase && r.variant == name)
            .map(|r| (r.after.0 - r.before.0, r.locked))
    };
    let mut names: Vec<&str> = Vec::new();
    for r in rows {
        if !names.contains(&r.variant.as_str()) && r.phase == "no-grab/moved" {
            names.push(&r.variant);
        }
    }
    let mut out = Vec::new();
    for n in names {
        let Some((none, _)) = dx("no-grab/moved", n) else {
            continue;
        };
        for g in ["grab/dragged", "grab/moved"] {
            let verdict = dx(g, n).map_or("not measured".to_string(), |(gd, locked)| {
                if locked {
                    format!(
                        "{:?} (no grab {none}, {g} {gd}, posted {posted})",
                        classify(none, gd, posted)
                    )
                } else {
                    format!(
                        "Inconclusive: burl's grab was not confirmed locked (no grab {none}, {g} {gd}, posted {posted})"
                    )
                }
            });
            out.push(format!("{n} vs {g}: {verdict}"));
        }
    }
    out
}

/// Rec. 601 luma of an RGB pixel.
fn luma(px: &[u8]) -> f64 {
    0.299 * f64::from(px[0]) + 0.587 * f64::from(px[1]) + 0.114 * f64::from(px[2])
}

/// A `side` x `side` px luma crop whose top-left is `margin` px up-left of the hot spot `(hx, hy)`
/// (the cursor sprite hangs down and right of its hot spot). `None` when it leaves the image.
pub fn crop_luma(img: &Image, hot: (f64, f64), side: usize, margin: usize) -> Option<Vec<f64>> {
    let x0 = hot.0.round() as i64 - margin as i64;
    let y0 = hot.1.round() as i64 - margin as i64;
    if x0 < 0 || y0 < 0 || x0 + side as i64 > img.w as i64 || y0 + side as i64 > img.h as i64 {
        return None;
    }
    let (x0, y0) = (x0 as usize, y0 as usize);
    let mut v = Vec::with_capacity(side * side);
    for y in y0..y0 + side {
        for x in x0..x0 + side {
            let i = (y * img.w + x) * 3;
            v.push(luma(&img.rgb[i..i + 3]));
        }
    }
    Some(v)
}

pub fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len().max(1) as f64
}

/// Mean absolute luma difference of two equal-length crops (0 = identical, 255 = inverted).
pub fn diff_score(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum::<f64>() / a.len().max(1) as f64
}

/// V5 numbers for one capture target (screen or window).
#[derive(Debug, Clone, PartialEq)]
pub struct ShotScores {
    pub pre_luma: f64,
    pub held_luma: f64,
    /// `-C` against no `-C`, before the press: is the sprite in the capture at all?
    pub sprite: f64,
    /// Two `-C` captures in a row before the press: the noise floor (burl animates).
    pub noise: f64,
    /// `-C` before the press against `-C` during the held press: did the sprite go?
    pub hide: f64,
    /// `-C` against no `-C` during the held press: is a sprite still drawn?
    pub held_sprite: f64,
}

/// Crop all five captures around `hot` (px) and score them. `None` if any crop leaves its image.
pub fn shot_scores(
    pre_c1: &Image,
    pre_c2: &Image,
    pre_nc: &Image,
    held_c: &Image,
    held_nc: &Image,
    hot: (f64, f64),
) -> Option<ShotScores> {
    let c = |i: &Image| crop_luma(i, hot, 48, 8);
    let (a, b, n, h, hn) = (c(pre_c1)?, c(pre_c2)?, c(pre_nc)?, c(held_c)?, c(held_nc)?);
    Some(ShotScores {
        pre_luma: mean(&a),
        held_luma: mean(&h),
        sprite: diff_score(&a, &n),
        noise: diff_score(&a, &b),
        hide: diff_score(&a, &h),
        held_sprite: diff_score(&h, &hn),
    })
}

/// What the V5 numbers allow us to say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hidden {
    /// `-C` makes no difference beyond noise: the sprite is not in captures, nothing can be asserted.
    NoSpriteInCaptures,
    /// The sprite shows before the press and the held crop differs by about as much: it went away.
    GoneWhileHeld,
    /// The sprite shows before the press and is still in the held capture: still drawn.
    StillDrawn,
    /// The numbers contradict each other: the held crop changed (`hide`) while the held
    /// cursor-versus-no-cursor comparison still shows the sprite (the background moved under a
    /// cursor that stayed), or the held comparison shows no sprite yet the crop did not change.
    Inconclusive,
}

/// The held capture is judged by its own cursor-versus-no-cursor comparison (`held_sprite`), not
/// by `hide` alone: `hide` also moves when the background under the crop changes.
pub fn hidden_verdict(s: &ShotScores) -> Hidden {
    let significant = |v: f64| v > 3.0 * s.noise.max(0.5);
    if !significant(s.sprite) {
        return Hidden::NoSpriteInCaptures;
    }
    let crop_changed = s.hide > 0.5 * s.sprite;
    match (significant(s.held_sprite), crop_changed) {
        (false, true) => Hidden::GoneWhileHeld,
        (true, false) => Hidden::StillDrawn,
        _ => Hidden::Inconclusive,
    }
}

// ---- process plumbing (a small copy of display_harness's, which is private) ----

struct Proc(Child);
impl Drop for Proc {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn poll<T>(what: &str, secs: u64, mut f: impl FnMut() -> Option<T>) -> Result<T, String> {
    let end = Instant::now() + Duration::from_secs(secs);
    loop {
        if let Some(v) = f() {
            return Ok(v);
        }
        if Instant::now() > end {
            return Err(format!("timed out after {secs}s waiting for {what}"));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}

fn sleep_ms(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

/// A finished (or timed out) helper run: output of stdout and stderr together.
struct Cap {
    status: Option<ExitStatus>,
    text: String,
}

impl Cap {
    fn ok(&self) -> bool {
        self.status.is_some_and(|s| s.success())
    }
    fn why(&self) -> String {
        self.status
            .as_ref()
            .map_or("timed out".to_string(), describe_exit)
    }
}

/// Runs `cmd` with a hard deadline; `Err` only when it cannot be spawned. Killed on timeout.
fn capture(cmd: &mut Command, secs: u64, out: &Path) -> Result<Cap, String> {
    let f = File::create(out).map_err(|e| format!("{}: {e}", out.display()))?;
    cmd.stdout(f.try_clone().map_err(|e| e.to_string())?)
        .stderr(f)
        .stdin(Stdio::null());
    let mut p = Proc(cmd.spawn().map_err(|e| format!("{cmd:?}: {e}"))?);
    let end = Instant::now() + Duration::from_secs(secs);
    loop {
        if let Some(st) = p.0.try_wait().map_err(|e| e.to_string())? {
            return Ok(Cap {
                status: Some(st),
                text: read(out),
            });
        }
        if Instant::now() > end {
            return Ok(Cap {
                status: None,
                text: read(out),
            });
        }
        sleep_ms(20);
    }
}

fn run_ok(cmd: &mut Command, secs: u64, out: &Path) -> Result<String, String> {
    let c = capture(cmd, secs, out)?;
    if !c.ok() {
        return Err(format!("{cmd:?}: {}\n{}", c.why(), c.text));
    }
    Ok(c.text)
}

struct Log(PathBuf);
impl Log {
    fn line(&self, s: &str) {
        eprintln!("{s}");
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&self.0) {
            let _ = writeln!(f, "{s}");
        }
    }
}

/// Releases the left button when dropped while down, so no failure leaves it pressed.
struct Button {
    bin: PathBuf,
    scratch: PathBuf,
    p: (i32, i32),
    down: bool,
}
impl Button {
    fn release(&mut self) {
        if !self.down {
            return;
        }
        self.down = false;
        let at = capture(Command::new(&self.bin).arg("pos"), 10, &self.scratch)
            .ok()
            .and_then(|c| parse_xy(&c.text))
            .unwrap_or(self.p);
        let _ = capture(
            Command::new(&self.bin).args(["up", &at.0.to_string(), &at.1.to_string()]),
            10,
            &self.scratch,
        );
    }
}
impl Drop for Button {
    fn drop(&mut self) {
        self.release();
    }
}

// ---- the lab ----

struct Ctx {
    dir: PathBuf,
    mouse: PathBuf,
    tapwatch: PathBuf,
    wid: PathBuf,
    burl_out: PathBuf,
    /// Where the cursor is placed: well inside burl's content area.
    p: (i32, i32),
    screen_pts: (i32, i32),
    log: Log,
    rows: RefCell<Vec<Row>>,
    tap_failed: Cell<bool>,
    /// Set by `with_grab`: burl logged `grab: event=on mode=locked` for the current press.
    locked: Cell<bool>,
    n: Cell<u32>,
}

impl Ctx {
    /// Bounded like every other helper call: a hung `wid` must not block the caller's own poll.
    fn window(&self) -> Option<Window> {
        let c = capture(
            Command::new(&self.wid).arg("burl"),
            10,
            &self.dir.join("wid.out"),
        )
        .ok()?;
        c.text.lines().find_map(parse_window)
    }

    fn mouse(&self, args: &[String]) -> Result<String, String> {
        run_ok(
            Command::new(&self.mouse).args(args),
            10,
            &self.dir.join("mouse.out"),
        )
    }

    fn cursor(&self) -> Result<(i32, i32), String> {
        let out = self.mouse(&["pos".into()])?;
        parse_xy(&out).ok_or(format!("mouse pos printed {out:?}"))
    }

    fn place(&self) -> Result<(), String> {
        self.mouse(&["warp".into(), self.p.0.to_string(), self.p.1.to_string()])?;
        sleep_ms(300);
        let at = self.cursor()?;
        if (at.0 - self.p.0).abs() > 3 || (at.1 - self.p.1).abs() > 3 {
            self.log.line(&format!(
                "note: could not place the cursor at {:?}, it is at {at:?}",
                self.p
            ));
        }
        Ok(())
    }

    /// Starts a listen-only observer; `None` when it cannot (recorded once, then not retried).
    fn tap_start(&self) -> Option<(Proc, PathBuf)> {
        if self.tap_failed.get() {
            return None;
        }
        self.n.set(self.n.get() + 1);
        let out = self.dir.join(format!("tap-{}.out", self.n.get()));
        let f = File::create(&out).ok()?;
        let child = Command::new(&self.tapwatch)
            .args(["3", "session"])
            .stdout(f.try_clone().ok()?)
            .stderr(f)
            .spawn()
            .ok()?;
        let mut p = Proc(child);
        let ready = poll("tapwatch", 5, || {
            let t = read(&out);
            if t.contains("ready") {
                return Some(true);
            }
            if t.contains("tap-create-failed") || p.0.try_wait().ok().flatten().is_some() {
                return Some(false);
            }
            None
        });
        if ready == Ok(true) {
            return Some((p, out));
        }
        self.tap_failed.set(true);
        self.log.line(&format!(
            "V4 tapwatch unavailable (CGEventTapCreate failed or helper died): {:?}; access: {:?}",
            read(&out).trim(),
            parse_access(&read(&out))
        ));
        None
    }

    fn tap_finish(&self, tap: (Proc, PathBuf)) -> TapSummary {
        let (mut p, out) = tap;
        let _ = poll("tapwatch to finish", 6, || p.0.try_wait().ok().flatten());
        summarize_tap(&parse_tap_events(&read(&out)))
    }

    /// One measured cell: cursor before, observer running, `post`, cursor after, `CGCursorIsVisible`.
    fn measure(
        &self,
        phase: &str,
        variant: &str,
        post: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<(), String> {
        let before = self.cursor()?;
        let tap = self.tap_start();
        let posted = post();
        sleep_ms(400);
        let after = self.cursor()?;
        let visible = self
            .mouse(&["vis".into()])
            .ok()
            .and_then(|t| parse_visible(&t));
        let tap = tap.map(|t| self.tap_finish(t));
        let row = Row {
            phase: phase.into(),
            variant: variant.into(),
            before,
            after,
            visible,
            tap,
            locked: !is_grab_phase(phase) || self.locked.get(),
        };
        // A row measured after a FAILED injection says nothing about the cursor (it "held still"
        // because nothing was posted): log it marked invalid and keep it out of the table and the
        // verdicts, so it can never read as a held cursor.
        match &posted {
            Ok(()) => {
                self.log.line(&format_row(&row));
                self.rows.borrow_mut().push(row);
            }
            Err(why) => self.log.line(&format_invalid_row(&row, why)),
        }
        posted
    }

    /// V1-V4 for every variant, then V2 once more right after a `warp` (does warp re-associate?).
    fn run_variants(&self, phase: &str, kind: &str) -> Result<(), String> {
        for (i, v) in variants().iter().enumerate() {
            let sign = if i % 2 == 0 { 1 } else { -1 };
            let base = self.cursor()?;
            self.measure(phase, v.name, &mut || {
                for k in 1..=EVENTS {
                    self.mouse(&variant_args(v, kind, base, k, sign))?;
                    sleep_ms(50);
                }
                Ok(())
            })?;
        }
        self.mouse(&["warp".into(), self.p.0.to_string(), self.p.1.to_string()])?;
        sleep_ms(300);
        let v = &variants()[1];
        let base = self.cursor()?;
        self.measure(phase, "V2 rel @hid after warp", &mut || {
            for k in 1..=EVENTS {
                self.mouse(&variant_args(v, kind, base, k, 1))?;
                sleep_ms(50);
            }
            Ok(())
        })
    }

    /// Press on the background, wait for burl's `grab: on`, run `f`, always release.
    fn with_grab(&self, f: &mut dyn FnMut() -> Result<(), String>) -> Result<(), String> {
        self.place()?;
        let seen = parse_grab_events(&read(&self.burl_out)).len();
        let mut button = Button {
            bin: self.mouse.clone(),
            scratch: self.dir.join("mouse-up.out"),
            p: self.p,
            down: false,
        };
        // Armed BEFORE the press: if the helper posts the left-down and then times out before it
        // exits, `?` returns early and only this guard can still release the button. An extra `up`
        // is harmless when the press was never posted.
        button.down = true;
        self.locked.set(false);
        self.mouse(&["down".into(), self.p.0.to_string(), self.p.1.to_string()])?;
        // The first `on` line after the press, with the mode burl reported (None until it arrives).
        let first_on = || {
            parse_grab_events(&read(&self.burl_out))
                .get(seen..)?
                .iter()
                .find(|(on, _)| *on)
                .map(|(_, mode)| mode.clone())
        };
        let on = |want: bool| {
            poll("a grab line", 15, || {
                parse_grab_events(&read(&self.burl_out))
                    .get(seen..)?
                    .iter()
                    .any(|(on, _)| *on == want)
                    .then_some(())
            })
        };
        match on(true) {
            Ok(()) => {
                let mode = first_on().unwrap_or_default();
                self.locked.set(mode == "locked");
                if mode == "locked" {
                    self.log.line("burl: grab on seen, mode=locked");
                } else {
                    self.log.line(&format!(
                        "burl: grab on seen but mode={mode:?}, not locked: lock verdicts for this press are inconclusive"
                    ));
                }
            }
            Err(e) => self.log.line(&format!(
                "burl: NO grab on after the press ({e}); measuring anyway, lock verdicts inconclusive"
            )),
        }
        sleep_ms(500);
        let r = f();
        button.release();
        match on(false) {
            Ok(()) => self.log.line("burl: grab off seen after the release"),
            Err(e) => self
                .log
                .line(&format!("burl: NO grab off after the release ({e})")),
        }
        r
    }

    fn shot(&self, name: &str, cursor: bool, win: Option<u32>) -> Result<Image, String> {
        let png = self.dir.join(format!("{name}.png"));
        let mut cmd = Command::new("screencapture");
        cmd.arg("-x");
        if cursor {
            cmd.arg("-C");
        }
        if let Some(id) = win {
            cmd.args(["-o", "-l", &id.to_string()]);
        }
        run_ok(cmd.arg(&png), 20, &self.dir.join("screencapture.out"))?;
        let bytes = std::fs::read(&png).map_err(|e| format!("{}: {e}", png.display()))?;
        crate::ssim::decode(&bytes)
    }

    /// V5, the part before the press: `-C` twice (noise floor) and once without `-C`.
    fn shots_pre(&self, tag: &str, win: Option<u32>) -> Result<[Image; 3], String> {
        Ok([
            self.shot(&format!("{tag}-pre-c1"), true, win)?,
            self.shot(&format!("{tag}-pre-c2"), true, win)?,
            self.shot(&format!("{tag}-pre-nc"), false, win)?,
        ])
    }

    /// The hot spot of the cursor in the capture's pixels (`origin` = the capture's top-left, points).
    fn hot(&self, img: &Image, origin: (i32, i32), width_pts: i32) -> (f64, f64) {
        let scale = img.w as f64 / f64::from(width_pts.max(1));
        (
            f64::from(self.p.0 - origin.0) * scale,
            f64::from(self.p.1 - origin.1) * scale,
        )
    }

    fn report_shots(
        &self,
        tag: &str,
        pre: &[Image; 3],
        held: Result<[Image; 2], String>,
        origin: (i32, i32),
        width_pts: i32,
    ) {
        let held = match held {
            Ok(h) => h,
            Err(e) => {
                return self
                    .log
                    .line(&format!("V5 {tag}: held captures failed: {e}"))
            }
        };
        let hot = self.hot(&pre[0], origin, width_pts);
        match shot_scores(&pre[0], &pre[1], &pre[2], &held[0], &held[1], hot) {
            Some(s) => self.log.line(&format!(
                "V5 {tag}: luma pre {:.1} held {:.1}; diff sprite(-C vs none, pre) {:.2}, noise(-C vs -C, pre) {:.2}, \
                 hide(pre vs held, -C) {:.2}, sprite-held(-C vs none, held) {:.2} => {:?}",
                s.pre_luma, s.held_luma, s.sprite, s.noise, s.hide, s.held_sprite, hidden_verdict(&s)
            )),
            None => self.log.line(&format!("V5 {tag}: the 48x48 crop at {hot:?} leaves the capture")),
        }
    }

    /// V6 for one binary: create the device, then (if it exists) post relative motion through it
    /// with no grab and under a held grab.
    fn hid_trial(&self, label: &str, bin: &Path) -> Result<(), String> {
        let out = self.dir.join(format!("{label}.out"));
        let f = File::create(&out).map_err(|e| e.to_string())?;
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(f.try_clone().map_err(|e| e.to_string())?)
            .stderr(f)
            .spawn()
            .map_err(|e| format!("{label}: {e}"))?;
        let mut stdin = child.stdin.take().ok_or("no stdin")?;
        let mut p = Proc(child);
        let ready = poll("the hid helper", 10, || {
            if read(&out).contains("ready") {
                return Some(Ok(()));
            }
            let st = p.0.try_wait().ok().flatten()?;
            Some(Err(describe_exit(&st)))
        });
        let text = read(&out);
        let note = match ready {
            Err(e) => Some(format!("V6 {label}: {e}; output {:?}", text.trim())),
            Ok(Err(why)) => Some(format!(
                "V6 {label}: helper ended before `ready`: {why}; output {:?}",
                text.trim()
            )),
            Ok(Ok(())) => {
                self.log.line(&format!(
                    "V6 {label}: {:?}",
                    text.trim().replace('\n', " | ")
                ));
                None
            }
        };
        if let Some(n) = note {
            self.log.line(&n);
            return Ok(());
        }
        if !text.contains("create=ok") {
            let end = poll("the hid helper to exit", 5, || {
                p.0.try_wait().ok().flatten()
            });
            self.log.line(&format!(
                "V6 {label}: no device; helper then: {:?}",
                end.map(|s| describe_exit(&s))
            ));
            return Ok(());
        }
        sleep_ms(2000);
        let sent = |n: usize| {
            poll("hid `sent`", 10, || {
                (read(&out).lines().filter(|l| l.starts_with("sent")).count() >= n).then_some(())
            })
        };
        let mut n = 0;
        let mut push = || -> Result<(), String> {
            n += 1;
            writeln!(stdin, "move {STEP} 0 {EVENTS}").map_err(|e| e.to_string())?;
            stdin.flush().map_err(|e| e.to_string())?;
            sent(n)?;
            // The helper prints `sent <actual>` even after a failed report, so a failure must be
            // read from `report-error`: otherwise a failed injection looks like "cursor held".
            if read(&out).contains("report-error") {
                return Err(format!(
                    "{label}: IOHIDUserDeviceHandleReport failed, this measurement is invalid: {:?}",
                    read(&out).trim().replace('\n', " | ")
                ));
            }
            Ok(())
        };
        self.place()?;
        let r = self.measure(&format!("{label}/no-grab"), "V6 hid rel", &mut push);
        if let Err(e) = r {
            self.log
                .line(&format!("V6 {label}: no-grab post failed: {e}"));
        }
        let r =
            self.with_grab(&mut || self.measure(&format!("{label}/grab"), "V6 hid rel", &mut push));
        if let Err(e) = r {
            self.log.line(&format!("V6 {label}: grab post failed: {e}"));
        }
        Ok(())
    }
}

fn sysinfo(dir: &Path, log: &Log) {
    for (name, args) in [
        ("sw_vers", &[][..]),
        ("csrutil", &["status"][..]),
        ("nvram", &["boot-args"][..]),
        ("uname", &["-m"][..]),
    ] {
        let r = capture(Command::new(name).args(args), 15, &dir.join("sysinfo.out"));
        log.line(&match r {
            Ok(c) => format!(
                "$ {name} {}: {}: {}",
                args.join(" "),
                c.why(),
                c.text.trim().replace('\n', " | ")
            ),
            Err(e) => format!("$ {name}: {e}"),
        });
    }
}

const ENTITLEMENTS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>com.apple.developer.hid.virtual.device</key><true/>
</dict></plist>
"#;

/// Runs the whole lab; `Err` only for an infrastructure failure.
pub fn run_lab(root: &Path) -> Result<(), String> {
    let dir = root.join("target/cursor-lab");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let log = Log(dir.join("lab.log"));
    let _ = std::fs::remove_file(&log.0);
    sysinfo(&dir, &log);

    let src = root.join("tools/display-harness");
    for (m, bin) in [
        ("mouse.m", "mouse"),
        ("wid.m", "wid"),
        ("tapwatch.m", "tapwatch"),
        ("hid.m", "hid"),
    ] {
        run_ok(
            Command::new("clang")
                .args(["-fobjc-arc", "-Wno-deprecated-declarations"])
                .args(["-framework", "Cocoa", "-framework", "CoreGraphics"])
                .args(["-framework", "ApplicationServices", "-framework", "IOKit"])
                .arg(src.join(m))
                .arg("-o")
                .arg(dir.join(bin)),
            120,
            &dir.join("clang.out"),
        )?;
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    eprintln!("+ cargo build --release -p burl");
    run_ok(
        Command::new(&cargo)
            .args(["build", "--release", "-p", "burl"])
            .current_dir(root),
        900,
        &dir.join("cargo.out"),
    )?;
    let target =
        std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);

    // As run_grab_check: nothing hidden but the mouse, so a click lands on the background and grabs.
    let burl_out = dir.join("burl.out");
    let out = File::create(&burl_out).map_err(|e| e.to_string())?;
    let _burl = Proc(
        Command::new(target.join("release/burl"))
            .arg(root.join("tests/golden/sim/small-repo.log"))
            .args([
                "--log-format",
                "custom",
                "-640x360",
                "--seconds-per-day",
                "10",
            ])
            .args([
                "--hash-seed",
                "31",
                "--prng",
                "glibc",
                "--disable-auto-skip",
            ])
            .args(["--disable-auto-rotate", "--log-level", "debug"])
            .args([
                "--hide",
                "tree,files,users,usernames,filenames,dirnames,bloom,progress",
            ])
            .stdout(out.try_clone().map_err(|e| e.to_string())?)
            .stderr(out)
            .spawn()
            .map_err(|e| format!("burl: {e}"))?,
    );
    let wid = dir.join("wid");
    let mut ctx = Ctx {
        dir: dir.clone(),
        mouse: dir.join("mouse"),
        tapwatch: dir.join("tapwatch"),
        wid,
        burl_out,
        p: (0, 0),
        screen_pts: (0, 0),
        log,
        rows: RefCell::new(Vec::new()),
        tap_failed: Cell::new(false),
        locked: Cell::new(false),
        n: Cell::new(0),
    };
    let w = poll("burl's window", 60, || ctx.window())?;
    run_ok(
        Command::new("osascript").args([
            "-e",
            "tell application \"System Events\" to set frontmost of (first process whose name is \"burl\") to true",
        ]),
        30,
        &dir.join("osascript.out"),
    )?;
    sleep_ms(3000);
    ctx.p = (w.x + (w.w as i32) / 2 - 100, w.y + (w.h as i32) * 3 / 4);
    ctx.screen_pts = parse_xy(&ctx.mouse(&["bounds".into()])?).ok_or("mouse bounds")?;
    ctx.log.line(&format!(
        "window {w:?}, cursor placed at {:?}, main display {:?} pt",
        ctx.p, ctx.screen_pts
    ));
    ctx.log.line(&header());

    // Phase 1: no grab. Motion alone grabs nothing (the `--grab` control).
    ctx.place()?;
    ctx.run_variants("no-grab/moved", "moved")?;

    // Phase 2: held grab, dragged events, with V5 captures around the press.
    let win = ctx.window().unwrap_or(w);
    let win_id = Some(win.id);
    ctx.place()?;
    let pre_screen = ctx.shots_pre("screen", None);
    let pre_win = ctx.shots_pre("window", win_id);
    let r = ctx.with_grab(&mut || {
        let held = |tag: &str, id: Option<u32>| -> Result<[Image; 2], String> {
            Ok([
                ctx.shot(&format!("{tag}-held-c"), true, id)?,
                ctx.shot(&format!("{tag}-held-nc"), false, id)?,
            ])
        };
        let (hs, hw) = (held("screen", None), held("window", win_id));
        match pre_screen.as_ref() {
            Ok(p) => ctx.report_shots("screen -C", p, hs, (0, 0), ctx.screen_pts.0),
            Err(e) => ctx
                .log
                .line(&format!("V5 screen: pre captures failed: {e}")),
        }
        match pre_win.as_ref() {
            Ok(p) => ctx.report_shots("window -C -l", p, hw, (win.x, win.y), win.w as i32),
            Err(e) => ctx
                .log
                .line(&format!("V5 window: pre captures failed: {e}")),
        }
        ctx.run_variants("grab/dragged", "dragged")
    });
    r?;

    // Phase 3: held grab, moved events (the press is still a left press; the type is the variable).
    ctx.with_grab(&mut || ctx.run_variants("grab/moved", "moved"))?;

    // V6: a virtual HID mouse, plain then ad-hoc signed with the restricted entitlement.
    if let Err(e) = ctx.hid_trial("hid-plain", &dir.join("hid")) {
        ctx.log.line(&format!("V6 hid-plain: {e}"));
    }
    let plist = dir.join("hid.entitlements");
    let signed = dir.join("hid-signed");
    std::fs::write(&plist, ENTITLEMENTS).map_err(|e| e.to_string())?;
    std::fs::copy(dir.join("hid"), &signed).map_err(|e| e.to_string())?;
    let cs = capture(
        Command::new("codesign")
            .args(["-s", "-", "-f", "--entitlements"])
            .arg(&plist)
            .arg(&signed),
        60,
        &dir.join("codesign.out"),
    )?;
    ctx.log
        .line(&format!("codesign: {}: {}", cs.why(), cs.text.trim()));
    if cs.ok() {
        if let Err(e) = ctx.hid_trial("hid-signed", &signed) {
            ctx.log.line(&format!("V6 hid-signed: {e}"));
        }
    }

    ctx.log.line("\n== table ==");
    ctx.log.line(&header());
    for r in ctx.rows.borrow().iter() {
        ctx.log.line(&format_row(r));
    }
    ctx.log.line("\n== per variant: no grab vs held grab ==");
    for l in summary(&ctx.rows.borrow()) {
        ctx.log.line(&l);
    }
    // Belt and braces: a button-up at the cursor, harmless if nothing is down.
    if let Ok(at) = ctx.cursor() {
        let _ = ctx.mouse(&["up".into(), at.0.to_string(), at.1.to_string()]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    fn img(w: usize, h: usize, f: impl Fn(usize, usize) -> u8) -> Image {
        let mut rgb = Vec::new();
        for y in 0..h {
            for x in 0..w {
                rgb.extend([f(x, y); 3]);
            }
        }
        Image { w, h, rgb }
    }

    #[test]
    fn parses_and_sums_tap_events() {
        let t = "access trusted=1 listen=1\nready\n\
                 event type=moved x=412.0 y=390.0 dx=10 dy=0\n\
                 event type=dragged x=422.0 y=390.0 dx=10 dy=-1\n\
                 event type=down x=1.0 y=2.0 dx=0 dy=0\nevent type=moved x=bad";
        let ev = parse_tap_events(t);
        assert_eq!(ev.len(), 3);
        let s = summarize_tap(&ev);
        assert_eq!((s.count, s.dx, s.dy), (2, 20, -1));
        assert_eq!(
            (s.first, s.last),
            (Some((412.0, 390.0)), Some((422.0, 390.0)))
        );
        assert_eq!(parse_access(t), Some((true, true)));
        assert_eq!(parse_access("tap-create-failed"), None);
    }

    #[test]
    fn parses_helper_output() {
        assert_eq!(parse_xy("412 390\n"), Some((412, 390)));
        assert_eq!(parse_xy("-5 7"), Some((-5, 7)));
        assert_eq!(parse_xy("nope"), None);
        assert_eq!(parse_visible("visible=1\n"), Some(true));
        assert_eq!(parse_visible("visible=0"), Some(false));
        assert_eq!(parse_visible("x"), None);
        assert_eq!(
            describe_exit(&ExitStatus::from_raw(9)),
            "killed by signal 9"
        );
        assert_eq!(describe_exit(&ExitStatus::from_raw(3 << 8)), "exit code 3");
    }

    #[test]
    fn builds_mouse_arguments_per_variant() {
        let vs = variants();
        assert_eq!(
            variant_args(&vs[0], "dragged", (400, 300), 2, 1),
            ["drag", "420", "300", "10", "0", "--tap", "hid"]
        );
        assert_eq!(
            variant_args(&vs[2], "moved", (400, 300), 2, -1),
            ["rel", "-10", "0", "--type", "moved", "--tap", "session"]
        );
        assert_eq!(
            variant_args(&vs[4], "moved", (0, 0), 1, 1),
            ["rel", "10", "0", "--type", "moved", "--tap", "hid", "--source", "hid"]
        );
    }

    #[test]
    fn classifies_what_the_cursor_did() {
        assert_eq!(classify(50, 0, 50), Effect::Detached);
        assert_eq!(classify(-50, 3, 50), Effect::Detached);
        assert_eq!(classify(50, 50, 50), Effect::IgnoresLock);
        assert_eq!(classify(0, 0, 50), Effect::Uninformative);
        assert_eq!(classify(10, 0, 50), Effect::Uninformative);
    }

    fn row(phase: &str, variant: &str, dx: i32) -> Row {
        Row {
            phase: phase.into(),
            variant: variant.into(),
            before: (100, 100),
            after: (100 + dx, 100),
            visible: Some(true),
            tap: None,
            locked: true,
        }
    }

    #[test]
    fn a_failed_injection_row_is_marked_invalid() {
        let r = row("grab/moved", "V6 hid rel", 0);
        let line = format_invalid_row(&r, "report failed");
        assert!(
            line.contains("INVALID") && line.contains("report failed"),
            "{line}"
        );
        assert!(line.contains("left out of the table"), "{line}");
    }

    #[test]
    fn a_held_row_without_a_confirmed_locked_grab_is_inconclusive() {
        // The cursor "stayed still" under a grab that never locked (both grab requests failed, or
        // it was confined, or no `on` line arrived): that must not read as Detached.
        let unlocked = Row {
            locked: false,
            ..row("grab/dragged", "A", 0)
        };
        let rows = [
            row("no-grab/moved", "A", 50),
            unlocked,
            row("grab/moved", "A", 50),
        ];
        let s = summary(&rows);
        assert!(
            s[0].starts_with("A vs grab/dragged: Inconclusive"),
            "{}",
            s[0]
        );
        assert!(!s[0].contains("Detached"));
        assert!(s[1].contains("IgnoresLock"), "{}", s[1]);
        assert!(is_grab_phase("grab/moved") && is_grab_phase("hid-signed/grab"));
        assert!(!is_grab_phase("no-grab/moved") && !is_grab_phase("hid-signed/no-grab"));
    }

    #[test]
    fn summary_compares_each_grab_row_with_no_grab() {
        let rows = [
            row("no-grab/moved", "A", 50),
            row("grab/dragged", "A", 0),
            row("grab/moved", "A", 50),
        ];
        let s = summary(&rows);
        assert_eq!(s.len(), 2);
        assert!(s[0].contains("Detached") && s[0].contains("grab/dragged"));
        assert!(s[1].contains("IgnoresLock"));
        assert!(format_row(&rows[0]).contains("tap unavailable"));
    }

    #[test]
    fn crops_scores_and_verdicts() {
        let flat = img(200, 200, |_, _| 100);
        // a bright 20x20 "sprite" at the hot spot (100,100)
        let sprite = img(200, 200, |x, y| {
            if (100..120).contains(&x) && (100..120).contains(&y) {
                255
            } else {
                100
            }
        });
        assert!(crop_luma(&flat, (3.0, 3.0), 48, 8).is_none());
        assert_eq!(
            crop_luma(&flat, (100.0, 100.0), 48, 8).unwrap().len(),
            48 * 48
        );
        let hot = (100.0, 100.0);
        // sprite in -C captures, gone while held
        let s = shot_scores(&sprite, &sprite, &flat, &flat, &flat, hot).unwrap();
        assert_eq!(s.noise, 0.0);
        assert!(s.sprite > 10.0 && s.hide > 10.0);
        assert_eq!(hidden_verdict(&s), Hidden::GoneWhileHeld);
        // sprite stays while held
        let s = shot_scores(&sprite, &sprite, &flat, &sprite, &flat, hot).unwrap();
        assert_eq!(hidden_verdict(&s), Hidden::StillDrawn);
        // the background changed under a cursor that stayed: the crop differs (`hide` is large)
        // but the held cursor-versus-no-cursor comparison still shows the sprite, so it is NOT
        // reported as gone
        let bg = img(200, 200, |_, _| 160);
        let sprite_on_bg = img(200, 200, |x, y| {
            if (100..120).contains(&x) && (100..120).contains(&y) {
                255
            } else {
                160
            }
        });
        let s = shot_scores(&sprite, &sprite, &flat, &sprite_on_bg, &bg, hot).unwrap();
        assert!(s.hide > 0.5 * s.sprite && s.held_sprite > 10.0, "{s:?}");
        assert_eq!(hidden_verdict(&s), Hidden::Inconclusive);
        // the held comparison shows no sprite and yet the crop did not change: also contradictory
        let s = shot_scores(&sprite, &sprite, &flat, &sprite, &sprite, hot).unwrap();
        assert_eq!(hidden_verdict(&s), Hidden::Inconclusive);
        // no sprite in captures at all
        let s = shot_scores(&flat, &flat, &flat, &flat, &flat, hot).unwrap();
        assert_eq!(hidden_verdict(&s), Hidden::NoSpriteInCaptures);
        assert_eq!(diff_score(&[0.0, 10.0], &[0.0, 0.0]), 5.0);
    }
}
