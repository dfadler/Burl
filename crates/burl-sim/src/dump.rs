//! The JSONL state dump, schema 1 (`reference/gource-cpp-dump/README.md`). Same record types
//! and key order as the fork's `dumpstate.cpp`. Floats print with Rust's shortest round-trip
//! form rather than `%.9g`; both parse back to the same f32, which is how the comparator reads
//! them. Non-finite values are the strings `"nan"`, `"inf"`, `"-inf"`.

use crate::pointer::PointerEvent;
use crate::sim::{Caption, Kind, Sim, SimError};
use burl_util::bounds::Bounds2D;
use std::fmt::Write;

pub const SCHEMA: u32 = 1;

/// Keys whose values are integers in the schema. The fork prints floats with `%.9g`, so `0`
/// and `1` look like integers in float fields; the comparator needs this list to tell an exact
/// i64 compare (timestamps do not fit an f32) from an f32 one.
pub const INT_KEYS: &[&str] = &[
    "schema",
    "section",
    "repo_count",
    "frame",
    "currtime",
    "lasttime",
    "commitqueue_len",
    "hash_seed",
    "rand_calls",
    "rand_last",
    "first_rand_frame",
    "loaded_frame0",
    "n_dirs",
    "n_files",
    "n_users",
    "n_removed_pending",
    "tag_seq",
    "visible_count",
    "depth",
    "removed_timestamp",
    "tagid",
    "queued",
    "active",
    "timestamp",
    "prng_seed",
    "every",
    "max_frames",
];

/// Float keys that depend on the platform libm (`sinf`/`cosf` in `calcFileDest`):
/// compared with a relative tolerance instead of bit equality (ticket #11 item 1).
pub const LIBM_KEYS: &[&str] = &["dest"];

/// Header keys describing the build, not the run: recorded, never compared.
pub const BUILD_KEYS: &[&str] = &["compiler", "char_signed", "fma_target", "flt_eval_method"];

fn flt(o: &mut String, v: f32) {
    if v.is_finite() {
        let _ = write!(o, "{v}");
    } else if v.is_nan() {
        o.push_str("\"nan\"");
    } else if v > 0.0 {
        o.push_str("\"inf\"");
    } else {
        o.push_str("\"-inf\"");
    }
}

fn arr(o: &mut String, v: &[f32]) {
    o.push('[');
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        flt(o, *x);
    }
    o.push(']');
}

fn bounds(o: &mut String, b: &Bounds2D) {
    o.push('[');
    for v in [b.min.x, b.min.y, b.max.x, b.max.y] {
        flt(o, v);
        o.push(',');
    }
    o.push_str(if b.first { "true]" } else { "false]" });
}

/// `jstr`: UTF-8 passes through, control characters become `\u00xx`.
fn jstr(o: &mut String, s: &str) {
    o.push('"');
    for c in s.chars() {
        match c {
            '"' | '\\' => {
                o.push('\\');
                o.push(c);
            }
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
}

/// `dumpCaptions`: `RCaption::dumpJson` per caption.
fn dump_captions<'a>(o: &mut String, caps: impl Iterator<Item = &'a Caption>) {
    o.push('[');
    for (i, c) in caps.enumerate() {
        if i > 0 {
            o.push(',');
        }
        o.push_str("{\"text\":");
        jstr(o, &c.text);
        let _ = write!(o, ",\"timestamp\":{},\"elapsed\":", c.timestamp);
        flt(o, c.elapsed);
        o.push_str(",\"alpha\":");
        flt(o, c.alpha);
        o.push('}');
    }
    o.push(']');
}

fn jbool(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}

/// `writeHeader`. `prng` is `"script"`, `"default"`, `"glibc"` or `"park"`.
pub fn header(prng: &str, seed: Option<u64>, every: u32, max_frames: u32) -> String {
    let mut o = String::new();
    let _ = write!(
        o,
        "{{\"schema\":{SCHEMA},\"type\":\"header\",\"prng\":\"{prng}\",\"prng_seed\":"
    );
    match seed {
        Some(s) => {
            let _ = write!(o, "{s}");
        }
        None => o.push_str("null"),
    }
    let _ = write!(
        o,
        ",\"every\":{every},\"max_frames\":{max_frames},\"compiler\":"
    );
    jstr(
        &mut o,
        &format!("burl {} (rustc)", env!("CARGO_PKG_VERSION")),
    );
    // Rust has no FMA contraction and no extended precision; `char` has no sign to speak of.
    o.push_str(",\"char_signed\":true,\"fma_target\":false,\"flt_eval_method\":0}\n");
    o
}

impl Sim {
    /// The `section` record (`dumpSectionBoundary`).
    pub fn section_record(&self) -> String {
        let mut o = String::new();
        let _ = write!(
            o,
            "{{\"schema\":{SCHEMA},\"type\":\"section\",\"section\":{},\"repo_count\":{},\"hash_seed\":{},\"stop_at_time\":",
            self.s.section, self.s.repo_count, self.s.hash_seed
        );
        flt(&mut o, self.s.stop_at_time);
        o.push_str(",\"stop_position\":");
        flt(&mut o, self.s.stop_position);
        o.push_str(",\"start_position\":");
        flt(&mut o, self.s.start_position);
        o.push_str(",\"path\":");
        jstr(&mut o, &self.s.path);
        o.push_str("}\n");
        o
    }

    /// Whether the frame just simulated is written, per the fork's `dumpState`: the first loaded
    /// frame, every `every`-th, and the last (finished, or the frame cap reached).
    pub fn should_dump(&self, every: u32, max_frames: u32) -> bool {
        self.framecount == self.loaded_frame0
            || self.framecount % every as i32 == 0
            || self.is_last_frame(max_frames)
    }

    pub fn is_last_frame(&self, max_frames: u32) -> bool {
        self.app_finished || (max_frames > 0 && self.framecount + 1 >= max_frames as i32)
    }

    /// The `frame` record for the frame just simulated.
    pub fn frame_record(&self) -> String {
        let mut o = String::with_capacity(4096);
        let _ = write!(
            o,
            "{{\"schema\":{SCHEMA},\"type\":\"frame\",\"section\":{},\"repo_count\":{},\"frame\":{},\"g\":{{",
            self.s.section, self.s.repo_count, self.framecount
        );
        let _ = write!(
            o,
            "\"currtime\":{},\"lasttime\":{}",
            self.currtime, self.lasttime
        );
        o.push_str(",\"subseconds\":");
        flt(&mut o, self.subseconds);
        o.push_str(",\"runtime\":");
        flt(&mut o, self.runtime);
        o.push_str(",\"idle_time\":");
        flt(&mut o, self.idle_time);
        let _ = write!(
            o,
            ",\"paused\":false,\"stop_position_reached\":{},\"commitqueue_len\":{}",
            jbool(self.stop_position_reached),
            self.commitqueue_len()
        );
        o.push_str(",\"last_percent\":");
        flt(&mut o, self.last_percent);
        o.push_str(",\"rotation_remaining_angle\":");
        flt(&mut o, self.rotation_remaining_angle);
        o.push_str(",\"rotate_angle\":");
        flt(&mut o, self.rotate_angle);
        let _ = write!(
            o,
            ",\"hash_seed\":{},\"rand_calls\":{},\"rand_last\":{},\"first_rand_frame\":{}",
            self.s.hash_seed,
            self.rand_calls(),
            self.rand_last() as u32,
            self.first_rand_frame
        );
        let _ = write!(
            o,
            ",\"loaded_frame0\":{},\"runtime_at_load\":",
            self.loaded_frame0
        );
        flt(&mut o, self.runtime_at_load);
        let _ = write!(
            o,
            ",\"n_dirs\":{},\"n_files\":{},\"n_users\":{},\"n_removed_pending\":{},\"tag_seq\":{}",
            self.dir_map.len(),
            self.file_map.len(),
            self.user_map.len(),
            self.removed.len(),
            self.tag_seq
        );
        o.push_str(",\"dir_bounds\":");
        bounds(&mut o, &self.dir_bounds);
        o.push_str(",\"user_bounds\":");
        bounds(&mut o, &self.user_bounds);
        if self.pointer_dump {
            self.dump_pointer(&mut o);
        }

        let c = &self.camera;
        o.push_str("},\"camera\":{\"pos\":");
        arr(&mut o, &c.pos.to_array());
        o.push_str(",\"dest\":");
        arr(&mut o, &c.dest.to_array());
        o.push_str(",\"target\":");
        arr(&mut o, &c.target.to_array());
        let _ = write!(o, ",\"lockon\":{},\"lockon_time\":", jbool(c.lockon));
        flt(&mut o, c.lockon_time);
        o.push('}');

        // std::map iteration order: dirs by abspath, files by fullpath, users by name
        o.push_str(",\"dirs\":[");
        for (i, &d) in self.dir_map.values().enumerate() {
            if i > 0 {
                o.push(',');
            }
            self.dump_dir(&mut o, d);
        }
        o.push_str("],\"files\":[");
        for (i, &f) in self.file_map.values().enumerate() {
            if i > 0 {
                o.push(',');
            }
            self.dump_file(&mut o, f);
        }
        o.push_str("],\"users\":[");
        for (i, &u) in self.user_map.values().enumerate() {
            if i > 0 {
                o.push(',');
            }
            self.dump_user(&mut o, u);
        }
        o.push_str("],\"captions\":{\"queued\":");
        dump_captions(&mut o, self.captions.iter());
        o.push_str(",\"active\":");
        dump_captions(&mut o, self.active_captions.iter());
        o.push_str("}}\n");
        o
    }

    /// `dumpPointerJson`: the pointer and selection state a pointer script drives.
    fn dump_pointer(&self, o: &mut String) {
        o.push_str(",\"pointer\":{\"mousepos\":");
        arr(o, &[self.mousepos.x, self.mousepos.y]);
        let _ = write!(
            o,
            ",\"buttons\":{},\"grab\":{},\"dragged\":{}",
            self.pointer_buttons,
            jbool(self.grab_mouse),
            jbool(self.mousedragged)
        );
        let name = |o: &mut String, key: &str, v: Option<String>| {
            let _ = write!(o, ",\"{key}\":");
            match v {
                Some(s) => jstr(o, &s),
                None => o.push_str("null"),
            }
        };
        let user = |u: Option<usize>| u.map(|u| self.user(u).name.clone());
        let file = |f: Option<usize>| f.map(|f| self.file(f).fullpath.clone());
        name(o, "hover_user", user(self.hover_user));
        name(o, "hover_file", file(self.hover_file));
        name(o, "selected_user", user(self.selected_user));
        name(o, "selected_file", file(self.selected_file));
        o.push('}');
    }

    fn dump_dir(&self, o: &mut String, d: usize) {
        let dd = self.dir(d);
        o.push_str("{\"path\":");
        jstr(o, &dd.abspath);
        o.push_str(",\"parent\":");
        match dd.parent {
            Some(p) => jstr(o, &self.dir(p).abspath),
            None => o.push_str("null"),
        }
        for (k, v) in [
            ("pos", dd.pos.to_array()),
            ("spos", dd.spos.to_array()),
            ("accel", dd.accel.to_array()),
            ("prev_accel", dd.prev_accel.to_array()),
        ] {
            let _ = write!(o, ",\"{k}\":");
            arr(o, &v);
        }
        for (k, v) in [
            ("dir_area", dd.dir_area),
            ("file_area", dd.file_area),
            ("dir_radius", dd.dir_radius),
            ("parent_radius", dd.parent_radius),
        ] {
            let _ = write!(o, ",\"{k}\":");
            flt(o, v);
        }
        let _ = write!(
            o,
            ",\"visible\":{},\"position_initialized\":{},\"visible_count\":{},\"depth\":{}",
            jbool(dd.visible),
            jbool(dd.position_initialized),
            dd.visible_count,
            dd.depth
        );
        for (k, v) in [
            ("since_node_visible", dd.since_node_visible),
            ("since_last_file_change", dd.since_last_file_change),
            ("since_last_node_change", dd.since_last_node_change),
        ] {
            let _ = write!(o, ",\"{k}\":");
            flt(o, v);
        }
        o.push_str(",\"col\":");
        arr(o, &dd.col);
        o.push_str(",\"quad_bounds\":");
        bounds(o, &dd.quad);
        o.push('}');
    }

    fn dump_file(&self, o: &mut String, f: usize) {
        let ff = self.file(f);
        o.push_str("{\"path\":");
        jstr(o, &ff.fullpath);
        o.push_str(",\"dir\":");
        match ff.dir {
            Some(d) => jstr(o, &self.dir(d).abspath),
            None => o.push_str("null"),
        }
        o.push_str(",\"pos\":");
        arr(o, &ff.pos.to_array());
        o.push_str(",\"dest\":");
        arr(o, &ff.dest.to_array());
        o.push_str(",\"distance\":");
        flt(o, ff.distance);
        o.push_str(",\"accel\":");
        arr(o, &ff.accel.to_array());
        o.push_str(",\"elapsed\":");
        flt(o, ff.elapsed);
        o.push_str(",\"alpha\":");
        flt(o, self.file_alpha(f));
        let _ = write!(
            o,
            ",\"hidden\":{},\"removing\":{},\"expired\":{},\"forced_removal\":{}",
            jbool(ff.hidden),
            jbool(ff.removing),
            jbool(ff.expired),
            jbool(ff.forced_removal)
        );
        o.push_str(",\"fade_start\":");
        flt(o, ff.fade_start);
        o.push_str(",\"last_action\":");
        flt(o, ff.last_action);
        let _ = write!(o, ",\"removed_timestamp\":{}", ff.removed_timestamp);
        o.push_str(",\"file_colour\":");
        arr(o, &ff.file_colour.to_array());
        o.push_str(",\"touch_colour\":");
        arr(o, &ff.touch_colour.to_array());
        o.push_str(",\"colour\":");
        arr(o, &self.file_colour(f).to_array());
        o.push_str(",\"size\":");
        flt(o, ff.size);
        o.push_str(",\"name_interval\":");
        flt(o, ff.name_interval);
        let _ = write!(o, ",\"tagid\":{}}}", ff.tagid);
    }

    fn dump_user(&self, o: &mut String, u: usize) {
        let uu = self.user(u);
        let idle = uu.active_actions.is_empty() && uu.actions.is_empty();
        o.push_str("{\"name\":");
        jstr(o, &uu.name);
        o.push_str(",\"pos\":");
        arr(o, &uu.pos.to_array());
        o.push_str(",\"accel\":");
        arr(o, &uu.accel.to_array());
        for (k, v) in [
            ("speed", uu.speed),
            ("size", uu.size),
            ("elapsed", uu.elapsed),
            ("alpha", self.user_alpha(u)),
            ("last_action", uu.last_action),
            ("action_interval", uu.action_interval),
            ("name_interval", uu.name_interval),
        ] {
            let _ = write!(o, ",\"{k}\":");
            flt(o, v);
        }
        o.push_str(",\"usercol\":");
        arr(o, &uu.usercol.to_array());
        let _ = write!(
            o,
            ",\"highlighted\":{},\"queued\":{},\"active\":{}",
            jbool(uu.highlighted),
            uu.actions.len(),
            uu.active_actions.len()
        );
        let _ = write!(
            o,
            ",\"idle\":{},\"fading\":{},\"inactive\":{},\"tagid\":{}",
            jbool(idle),
            jbool(idle && (uu.elapsed - uu.last_action) > self.s.user_idle_time),
            jbool(idle && (uu.elapsed - uu.last_action) as f64 > 10.0),
            uu.tagid
        );
        o.push_str(",\"actions\":[");
        let mut first = true;
        for (active, list) in [(false, &uu.actions), (true, &uu.active_actions)] {
            for a in list {
                if !first {
                    o.push(',');
                }
                first = false;
                o.push_str("{\"file\":");
                jstr(o, &self.file(a.target).fullpath);
                let kind = match a.kind {
                    Kind::Create => "A",
                    Kind::Remove => "D",
                    Kind::Modify => "M",
                };
                let _ = write!(o, ",\"kind\":\"{kind}\",\"timestamp\":{}", a.timestamp);
                for (k, v) in [("t", a.t), ("progress", a.progress), ("rate", a.rate)] {
                    let _ = write!(o, ",\"{k}\":");
                    flt(o, v);
                }
                let _ = write!(o, ",\"active\":{}}}", jbool(active));
            }
        }
        o.push_str("]}");
    }
}

/// What the hidden `--debug-dump-*` / `--prng` options select.
pub struct DumpOptions {
    pub every: u32,
    /// 0 = no cap.
    pub max_frames: u32,
    pub prng: String,
    pub prng_seed: Option<u64>,
    /// The scripted pointer events (`--debug-pointer-script`); `None` = no `g.pointer` object.
    pub pointer: Option<Vec<PointerEvent>>,
}

/// The dump header, once per run (the fork writes it at the first section boundary).
pub fn write_header(out: &mut dyn std::io::Write, o: &DumpOptions) -> Result<(), SimError> {
    out.write_all(header(&o.prng, o.prng_seed, o.every, o.max_frames).as_bytes())
        .map_err(write_err)
}

fn write_err(e: std::io::Error) -> SimError {
    SimError(format!("could not write the dump: {e}"))
}

impl Sim {
    /// Run one section to its end, writing its section record and every selected frame
    /// (`Gource::update` with the fork's hook after `logic`). The frame counter is the Sim's
    /// own, so it restarts with each section's Sim.
    pub fn run_section(
        &mut self,
        out: &mut dyn std::io::Write,
        o: &DumpOptions,
    ) -> Result<(), SimError> {
        out.write_all(self.section_record().as_bytes())
            .map_err(write_err)?;
        self.pointer_dump = o.pointer.is_some();
        let mut next_event = 0;
        while !self.app_finished {
            self.update()?;
            if self.should_dump(o.every, o.max_frames) {
                out.write_all(self.frame_record().as_bytes())
                    .map_err(write_err)?;
            }
            // scripted events land after the dump and before the draw: a click on frame N is
            // first seen in the dump of frame N+1
            if let Some(events) = &o.pointer {
                while let Some(e) = events
                    .get(next_event)
                    .filter(|e| e.frame <= self.framecount)
                {
                    self.apply_pointer(e);
                    next_event += 1;
                }
                // the draw: `mousetrace`, then `cursor.logic` with the fixed tick
                self.draw_pointer();
                self.tick_cursor(self.max_tick_rate);
            }
            // the cap (or the natural end) finishes the run after its last frame is written
            if self.is_last_frame(o.max_frames) {
                self.app_finished = true;
            }
            self.end_frame();
        }
        out.flush().map_err(write_err)
    }

    /// A single-section run: the header, then the section.
    pub fn run_dump(
        &mut self,
        out: &mut dyn std::io::Write,
        o: &DumpOptions,
    ) -> Result<(), SimError> {
        write_header(out, o)?;
        self.run_section(out, o)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_finite_floats_are_strings_and_the_header_has_the_schema_and_mode() {
        let mut o = String::new();
        for v in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.0, 1.5] {
            flt(&mut o, v);
            o.push(' ');
        }
        assert_eq!(o, "\"nan\" \"inf\" \"-inf\" -0 1.5 ");
        let h = header("script", None, 5, 4000);
        assert!(h.starts_with(
            "{\"schema\":1,\"type\":\"header\",\"prng\":\"script\",\"prng_seed\":null,\"every\":5,"
        ));
        assert!(h.ends_with("}\n"));
    }

    #[test]
    fn strings_escape_like_the_fork() {
        let mut o = String::new();
        jstr(&mut o, "a\"b\\c\u{1}é");
        assert_eq!(o, "\"a\\\"b\\\\c\\u0001é\"");
    }
}
