//! Typed settings and their import from a `ConfFile`: `SDLAppSettings::importDisplaySettings`
//! (core/settings.cpp) and `GourceSettings::importGourceSettings` (gource_settings.cpp:611),
//! with the C++ validation order and messages. Imports return `ConfError` instead of throwing.
//!
//! Not ported (see the PR): macOS NFC normalisation of `user-image-dir` names, hex-float
//! `atof` input, and the stdin wait (the binary does it). `start-position random` needs the
//! PRNG, so it records `start_position_random` and leaves the value 0 for the sim lanes.

use crate::args::{ArgType, ARG_TYPES};
use crate::conffile::{ConfEntry, ConfError, ConfFile, ConfSection};
use burl_util::cstr::atoi;
use burl_util::regex::Regex;
use burl_util::strhash::DEFAULT_HASH_SEED;
use burl_util::timeparse::parse_date_time;
use std::collections::BTreeMap;
use std::sync::LazyLock;

static RECT: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^([0-9.]+)x([0-9.]+)$"));
static VIEWPORT: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^([0-9.]+)x([0-9.]+)(!)?$"));

/// `parseRectangle`: both numbers `atoi`, zero allowed.
fn parse_rectangle(value: &str) -> Option<(i32, i32)> {
    let m = RECT.captures(value.as_bytes())?;
    Some((atoi(&m[0]), atoi(&m[1])))
}

#[derive(Clone, Debug, PartialEq)]
pub struct DisplaySettings {
    pub width: i32,
    pub height: i32,
    pub viewport_specified: bool,
    pub fullscreen: bool,
    pub frameless: bool,
    pub multisample: bool,
    pub transparent: bool,
    pub resizable: bool,
    pub vsync: bool,
    pub high_dpi: bool,
    pub screen: i32,
    pub window_x: i32,
    pub window_y: i32,
    pub output_ppm_filename: String,
    pub output_framerate: i32,
}

impl Default for DisplaySettings {
    /// `setDisplayDefaults` (1024x640 on macOS, 1024x768 elsewhere).
    fn default() -> Self {
        DisplaySettings {
            width: 1024,
            height: if cfg!(target_os = "macos") { 640 } else { 768 },
            viewport_specified: false,
            fullscreen: false,
            frameless: false,
            multisample: false,
            transparent: false,
            resizable: true,
            vsync: true,
            high_dpi: false,
            screen: -1,
            window_x: -1,
            window_y: -1,
            output_ppm_filename: String::new(),
            output_framerate: 60,
        }
    }
}

/// `importDisplaySettings`: defaults, then the first `[display]` section only.
pub fn import_display_settings(conf: &ConfFile) -> Result<DisplaySettings, ConfError> {
    let mut d = DisplaySettings::default();
    let Some(sec) = conf.section("display") else {
        return Ok(d);
    };

    if let Some(e) = sec.entry("viewport") {
        let m = VIEWPORT.captures(e.value.as_bytes());
        match m {
            Some(m) if atoi(&m[0]) > 0 && atoi(&m[1]) > 0 => {
                d.width = atoi(&m[0]);
                d.height = atoi(&m[1]);
                if m.len() > 2 {
                    d.resizable = false;
                }
                d.viewport_specified = true;
            }
            _ => return Err(conf.invalid_value_error(e)),
        }
    }
    if let Some(e) = sec.entry("window-position") {
        match parse_rectangle(&e.value) {
            Some((x, y)) => (d.window_x, d.window_y) = (x, y),
            None => return Err(conf.invalid_value_error(e)),
        }
    }
    if let Some(e) = sec.entry("screen") {
        d.screen = e.get_int();
        if d.screen < 1 {
            return Err(conf.invalid_value_error(e));
        }
    }
    d.multisample = sec.get_bool("multi-sampling");
    if sec.get_bool("fullscreen") {
        d.fullscreen = true;
    }
    if sec.get_bool("windowed") {
        d.fullscreen = false;
    }
    if sec.get_bool("frameless") && !d.fullscreen {
        d.frameless = true;
    }
    if d.fullscreen && !d.viewport_specified {
        d.width = 0;
        d.height = 0;
    }
    d.transparent = sec.get_bool("transparent");
    if sec.get_bool("no-vsync") {
        d.vsync = false;
    }
    d.high_dpi = sec.get_bool("high-dpi");
    if let Some(e) = sec.entry("output-ppm-stream") {
        if !e.has_value() {
            return Err(conf.entry_error(e, "specify ppm output file or '-' for stdout"));
        }
        d.output_ppm_filename = e.value.clone();
    }
    if let Some(e) = sec.entry("output-framerate") {
        if !e.has_value() {
            return Err(conf.entry_error(e, "specify framerate (25,30,60)"));
        }
        d.output_framerate = e.get_int();
        if !matches!(d.output_framerate, 25 | 30 | 60) {
            return Err(conf.entry_error(e, "supported framerates are 25,30,60"));
        }
    }
    Ok(d)
}

/// The per-repo settings (`setGourceDefaults` fields). `Default` is the C++ default set;
/// `import_gource_settings` resets to it for every section, so nothing leaks between repos.
#[derive(Clone, Debug)]
pub struct GourceSettings {
    pub path: String,
    pub default_path: bool,
    pub hide_date: bool,
    pub hide_users: bool,
    pub hide_tree: bool,
    pub hide_files: bool,
    pub hide_usernames: bool,
    pub hide_filenames: bool,
    pub hide_dirnames: bool,
    pub hide_progress: bool,
    pub hide_bloom: bool,
    pub hide_mouse: bool,
    pub hide_root: bool,
    pub start_timestamp: i64,
    pub start_date: String,
    pub stop_timestamp: i64,
    pub stop_date: String,
    pub start_position: f32,
    /// `--start-position random`: the value is drawn from the PRNG by the sim lanes.
    pub start_position_random: bool,
    pub stop_position: f32,
    pub stop_at_time: f32,
    pub stop_at_end: bool,
    pub dont_stop: bool,
    pub no_time_travel: bool,
    pub fixed_user_size: bool,
    pub author_time: bool,
    pub show_key: bool,
    pub disable_auto_rotate: bool,
    pub disable_input: bool,
    pub auto_skip_seconds: f32,
    pub days_per_second: f32,
    pub file_idle_time: f32,
    pub file_idle_time_at_end: f32,
    pub time_scale: f32,
    pub loop_: bool,
    pub loop_delay_seconds: f32,
    pub logo: String,
    pub logo_offset: [f32; 2],
    pub colour_user_images: bool,
    pub default_user_image: String,
    pub user_image_dir: String,
    pub user_image_map: BTreeMap<String, String>,
    pub camera_zoom_min: f32,
    pub camera_zoom_default: f32,
    pub camera_zoom_max: f32,
    pub camera_mode: String,
    pub padding: f32,
    pub crop_vertical: bool,
    pub crop_horizontal: bool,
    pub bloom_multiplier: f32,
    pub bloom_intensity: f32,
    pub background_colour: [f32; 3],
    pub background_image: String,
    pub title: String,
    pub font_scale: f32,
    pub default_font_scale: bool,
    pub font_file: String,
    pub font_size: i32,
    pub filename_font_size: i32,
    pub dirname_font_size: i32,
    pub user_font_size: i32,
    pub scaled_font_size: i32,
    pub scaled_user_font_size: i32,
    pub scaled_dirname_font_size: i32,
    pub scaled_filename_font_size: i32,
    pub dir_colour: [f32; 3],
    pub font_colour: [f32; 3],
    pub highlight_colour: [f32; 3],
    pub selection_colour: [f32; 3],
    pub dir_name_depth: i32,
    pub dir_name_position: f32,
    pub elasticity: f32,
    pub git_branch: String,
    pub log_format: String,
    pub date_format: String,
    pub max_files: i32,
    pub max_user_speed: f32,
    pub max_file_lag: f32,
    pub user_idle_time: f32,
    pub user_friction: f32,
    pub user_scale: f32,
    pub follow_users: Vec<String>,
    pub highlight_users: Vec<String>,
    pub highlight_all_users: bool,
    pub highlight_dirs: bool,
    pub caption_file: String,
    pub caption_duration: f32,
    pub caption_size: i32,
    pub caption_offset: i32,
    pub caption_colour: [f32; 3],
    pub filename_colour: [f32; 3],
    pub filename_time: f32,
    /// `gStringHashSeed`, reset to 31 with every section import.
    pub hash_seed: i32,
    pub file_filters: Vec<Regex>,
    pub file_show_filters: Vec<Regex>,
    pub user_filters: Vec<Regex>,
    pub user_show_filters: Vec<Regex>,
    pub file_extensions: bool,
    pub file_extension_fallback: bool,
}

impl Default for GourceSettings {
    fn default() -> Self {
        let mut s = GourceSettings {
            path: ".".into(),
            default_path: true,
            hide_date: false,
            hide_users: false,
            hide_tree: false,
            hide_files: false,
            hide_usernames: false,
            hide_filenames: false,
            hide_dirnames: false,
            hide_progress: false,
            hide_bloom: false,
            hide_mouse: false,
            hide_root: false,
            start_timestamp: 0,
            start_date: String::new(),
            stop_timestamp: 0,
            stop_date: String::new(),
            start_position: 0.0,
            start_position_random: false,
            stop_position: 0.0,
            stop_at_time: -1.0,
            stop_at_end: false,
            dont_stop: false,
            no_time_travel: false,
            fixed_user_size: false,
            author_time: false,
            show_key: false,
            disable_auto_rotate: false,
            disable_input: false,
            auto_skip_seconds: 3.0,
            days_per_second: 0.1,
            file_idle_time: 0.0,
            file_idle_time_at_end: 0.0,
            time_scale: 1.0,
            loop_: false,
            loop_delay_seconds: 3.0,
            logo: String::new(),
            logo_offset: [20.0, 20.0],
            colour_user_images: false,
            default_user_image: String::new(),
            user_image_dir: String::new(),
            user_image_map: BTreeMap::new(),
            camera_zoom_min: 50.0,
            camera_zoom_default: 100.0,
            camera_zoom_max: 10000.0,
            camera_mode: "overview".into(),
            padding: 1.1,
            crop_vertical: false,
            crop_horizontal: false,
            bloom_multiplier: 1.0,
            bloom_intensity: 0.75,
            background_colour: [0.1, 0.1, 0.1],
            background_image: String::new(),
            title: String::new(),
            font_scale: 1.0,
            default_font_scale: true,
            font_file: "FreeSans.ttf".into(),
            font_size: 16,
            filename_font_size: 14,
            dirname_font_size: 14,
            user_font_size: 14,
            scaled_font_size: 0,
            scaled_user_font_size: 0,
            scaled_dirname_font_size: 0,
            scaled_filename_font_size: 0,
            dir_colour: [1.0; 3],
            font_colour: [1.0; 3],
            highlight_colour: [1.0; 3],
            selection_colour: [1.0, 1.0, 0.3],
            dir_name_depth: 0,
            dir_name_position: 0.5,
            elasticity: 0.0,
            git_branch: String::new(),
            log_format: String::new(),
            date_format: "%A, %d %B, %Y %X".into(),
            max_files: 0,
            max_user_speed: 500.0,
            max_file_lag: 5.0,
            user_idle_time: 3.0,
            user_friction: 1.0,
            user_scale: 1.0,
            follow_users: Vec::new(),
            highlight_users: Vec::new(),
            highlight_all_users: false,
            highlight_dirs: false,
            caption_file: String::new(),
            caption_duration: 10.0,
            caption_size: 16,
            caption_offset: 0,
            caption_colour: [1.0; 3],
            filename_colour: [1.0; 3],
            filename_time: 4.0,
            hash_seed: DEFAULT_HASH_SEED,
            file_filters: Vec::new(),
            file_show_filters: Vec::new(),
            user_filters: Vec::new(),
            user_show_filters: Vec::new(),
            file_extensions: false,
            file_extension_fallback: false,
        };
        s.set_scaled_font_sizes();
        s
    }
}

/// `sscanf("%02x%02x%02x")` over a 6-character string: whitespace skipped before each field,
/// an optional sign, at most two hex digits per field.
fn scan_hex_colour(s: &str) -> Option<[i32; 3]> {
    let b = s.as_bytes();
    let mut pos = 0;
    let mut out = [0i32; 3];
    for slot in &mut out {
        while pos < b.len() && b[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let mut width = 2;
        let mut neg = false;
        if pos < b.len() && (b[pos] == b'+' || b[pos] == b'-') {
            neg = b[pos] == b'-';
            pos += 1;
            width -= 1;
        }
        let mut v: i32 = 0;
        let mut n = 0;
        while n < width && pos < b.len() && b[pos].is_ascii_hexdigit() {
            v = v * 16 + (b[pos] as char).to_digit(16).unwrap() as i32;
            pos += 1;
            n += 1;
        }
        if n == 0 {
            return None;
        }
        *slot = if neg { -v } else { v };
    }
    Some(out)
}

impl GourceSettings {
    /// `setScaledFontSizes`: `clamp((int)(size * scale), 1, 100)`, the product taken in f32.
    pub fn set_scaled_font_sizes(&mut self) {
        let scale = |size: i32| ((size as f32 * self.font_scale) as i32).clamp(1, 100);
        self.scaled_font_size = scale(self.font_size);
        self.scaled_user_font_size = scale(self.user_font_size);
        self.scaled_dirname_font_size = scale(self.dirname_font_size);
        self.scaled_filename_font_size = scale(self.filename_font_size);
    }

    /// The per-repo fixups `GourceShell::getNext` applies after each import.
    pub fn apply_shell_fixups(&mut self, repo_count: usize, exporting: bool) {
        if exporting && !(self.dont_stop || self.loop_ || self.path == "-") {
            self.stop_at_end = true;
        }
        if repo_count > 1 && self.stop_at_time <= 0.0 && self.stop_position <= 0.0 {
            self.stop_at_time = 60.0;
        }
    }
}

/// A required-value check: empty value is an error with the given text.
fn need<'a>(
    conf: &ConfFile,
    sec: &'a ConfSection,
    key: &str,
    msg: &str,
) -> Result<Option<&'a ConfEntry>, ConfError> {
    match sec.entry(key) {
        Some(e) if !e.has_value() => Err(conf.entry_error(e, msg)),
        e => Ok(e),
    }
}

fn colour(conf: &ConfFile, e: &ConfEntry) -> Result<[f32; 3], ConfError> {
    if e.is_vec3() {
        return Ok(e.get_vec3());
    }
    if e.value.len() == 6 {
        if let Some([r, g, b]) = scan_hex_colour(&e.value) {
            return Ok([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]);
        }
    }
    Err(conf.invalid_value_error(e))
}

/// The four `--*-filter` multi-value lists (user regexes, unanchored, flags 0).
fn filters(conf: &ConfFile, sec: &ConfSection, key: &str) -> Result<Vec<Regex>, ConfError> {
    let mut out = Vec::new();
    for e in sec.entries(key) {
        if !e.has_value() {
            return Err(conf.entry_error(e, &format!("specify {key} (regex)")));
        }
        match Regex::new_user(&e.value) {
            Ok(r) => out.push(r),
            Err(_) => return Err(conf.entry_error(e, &format!("invalid {key} regular expression"))),
        }
    }
    Ok(out)
}

fn strings(conf: &ConfFile, sec: &ConfSection, key: &str) -> Result<Vec<String>, ConfError> {
    let mut out = Vec::new();
    for e in sec.entries(key) {
        if !e.has_value() {
            return Err(conf.entry_error(e, &format!("specify {key} (user)")));
        }
        out.push(e.value.clone());
    }
    Ok(out)
}

/// `--git-branch` validation `^(?!-)[/\w.,;_=+{}\[\]-]+$` without look-ahead.
fn valid_git_branch(b: &str) -> bool {
    !b.is_empty()
        && !b.starts_with('-')
        && b.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"/._,;=+{}[]-".contains(&c))
}

fn local_date(ts: i64) -> String {
    use chrono::TimeZone;
    chrono::Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|t| t.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

/// `importGourceSettings` for one `[gource]` section, defaults first.
pub fn import_gource_settings(
    conf: &ConfFile,
    sec: &ConfSection,
) -> Result<GourceSettings, ConfError> {
    let mut s = GourceSettings::default();

    // hide flags
    let mut hide_fields: Vec<String> = Vec::new();
    if let Some(e) = sec.entry("hide") {
        if !e.has_value() {
            return Err(conf.missing_value_error(e));
        }
        let mut rest = e.value.as_str();
        while let Some(sep) = rest.find(',') {
            if sep == 0 && rest.len() == 1 {
                break;
            }
            if sep == 0 {
                rest = &rest[1..];
                continue;
            }
            hide_fields.push(rest[..sep].to_string());
            rest = &rest[sep + 1..];
        }
        if !rest.is_empty() && rest != "," {
            hide_fields.push(rest.to_string());
        }
        const FIELDS: [&str; 11] = [
            "date",
            "users",
            "tree",
            "files",
            "usernames",
            "filenames",
            "dirnames",
            "bloom",
            "progress",
            "mouse",
            "root",
        ];
        for f in &hide_fields {
            if !FIELDS.contains(&f.as_str()) {
                return Err(conf.entry_error(e, &format!("unknown option hide {f}")));
            }
        }
    }
    for (name, ty) in ARG_TYPES {
        if let (Some(field), ArgType::Bool) = (name.strip_prefix("hide-"), ty) {
            if sec.get_bool(name) {
                hide_fields.push(field.to_string());
            }
        }
    }
    for f in &hide_fields {
        match f.as_str() {
            "date" => s.hide_date = true,
            "users" => s.hide_users = true,
            "tree" => s.hide_tree = true,
            "files" => s.hide_files = true,
            "usernames" => s.hide_usernames = true,
            "filenames" => s.hide_filenames = true,
            "dirnames" => s.hide_dirnames = true,
            "bloom" => s.hide_bloom = true,
            "progress" => s.hide_progress = true,
            "root" => s.hide_root = true,
            "mouse" => {
                s.hide_mouse = true;
                s.hide_progress = true;
            }
            _ => {}
        }
    }

    if let Some(e) = sec.entry("date-format") {
        if !e.has_value() {
            return Err(conf.missing_value_error(e));
        }
        s.date_format = e.value.clone();
    }
    s.disable_auto_rotate = sec.get_bool("disable-auto-rotate");
    if sec.get_bool("disable-auto-skip") {
        s.auto_skip_seconds = -1.0;
    }
    s.disable_input = sec.get_bool("disable-input");
    s.loop_ = sec.get_bool("loop");

    if let Some(e) = need(
        conf,
        sec,
        "loop-delay-seconds",
        "specify loop-delay-seconds (float)",
    )? {
        s.loop_delay_seconds = e.get_float();
        if s.loop_delay_seconds <= 0.0 {
            return Err(conf.invalid_value_error(e));
        }
    }
    if let Some(e) = sec.entry("git-branch") {
        if !e.has_value() {
            return Err(conf.missing_value_error(e));
        }
        if !valid_git_branch(&e.value) {
            return Err(conf.invalid_value_error(e));
        }
        s.git_branch = e.value.clone();
    }
    s.colour_user_images = sec.get_bool("colour-images");
    if let Some(e) = need(conf, sec, "crop", "specify crop (vertical,horizontal)")? {
        match e.value.as_str() {
            "vertical" => s.crop_vertical = true,
            "horizontal" => s.crop_horizontal = true,
            _ => return Err(conf.invalid_value_error(e)),
        }
    }
    if let Some(e) = need(conf, sec, "log-format", "specify log-format (format)")? {
        s.log_format = e.value.clone();
        if s.log_format == "cvs" {
            return Err(conf.entry_error(e, "please use either 'cvs2cl' or 'cvs-exp'"));
        }
        if !matches!(
            s.log_format.as_str(),
            "git" | "cvs-exp" | "cvs2cl" | "svn" | "custom" | "hg" | "bzr" | "apache"
        ) {
            return Err(conf.invalid_value_error(e));
        }
    }
    if let Some(e) = need(
        conf,
        sec,
        "default-user-image",
        "specify default-user-image (image path)",
    )? {
        s.default_user_image = e.value.clone();
    }
    if let Some(e) = need(
        conf,
        sec,
        "user-image-dir",
        "specify user-image-dir (directory)",
    )? {
        s.user_image_dir = e.value.clone();
        if !s.user_image_dir.ends_with('/') {
            s.user_image_dir.push('/');
        }
        if !std::path::Path::new(&s.user_image_dir).is_dir() {
            return Err(conf.entry_error(e, "specified user-image-dir is not a directory"));
        }
        let rd = std::fs::read_dir(&s.user_image_dir)
            .map_err(|_| conf.entry_error(e, "error reading specified user-image-dir"))?;
        for ent in rd {
            let ent =
                ent.map_err(|_| conf.entry_error(e, "error reading specified user-image-dir"))?;
            let file = ent.file_name().to_string_lossy().into_owned();
            let ext = file
                .rfind('.')
                .map(|i| file[i..].to_ascii_lowercase())
                .unwrap_or_default();
            if ext != ".jpg" && ext != ".jpeg" && ext != ".png" {
                continue;
            }
            let name = file[..file.len() - ext.len()].to_string();
            s.user_image_map
                .insert(name, format!("{}{}", s.user_image_dir, file));
        }
    }
    if let Some(e) = need(conf, sec, "caption-file", "specify caption file (filename)")? {
        s.caption_file = e.value.clone();
        if !std::path::Path::new(&s.caption_file).exists() {
            return Err(conf.entry_error(e, "caption file not found"));
        }
    }
    if let Some(e) = need(
        conf,
        sec,
        "caption-duration",
        "specify caption duration (seconds)",
    )? {
        s.caption_duration = e.get_float();
        if s.caption_duration <= 0.0 {
            return Err(conf.invalid_value_error(e));
        }
    }
    if let Some(e) = need(conf, sec, "caption-size", "specify caption size")? {
        s.caption_size = e.get_int();
        if !(1..=100).contains(&s.caption_size) {
            return Err(conf.invalid_value_error(e));
        }
    }
    if let Some(e) = need(conf, sec, "caption-offset", "specify caption offset")? {
        s.caption_offset = e.get_int();
    }
    if let Some(e) = need(
        conf,
        sec,
        "caption-colour",
        "specify caption colour (FFFFFF)",
    )? {
        s.caption_colour = colour(conf, e)?;
    }
    if let Some(e) = need(
        conf,
        sec,
        "filename-colour",
        "specify filename colour (FFFFFF)",
    )? {
        s.filename_colour = colour(conf, e)?;
    }
    if let Some(e) = need(
        conf,
        sec,
        "filename-time",
        "specify duration to keep files on screen (float)",
    )? {
        s.filename_time = e.get_float();
        if s.filename_time < 2.0 {
            return Err(conf.entry_error(e, "filename-time must be >= 2.0"));
        }
    }
    if let Some(e) = need(
        conf,
        sec,
        "bloom-intensity",
        "specify bloom-intensity (float)",
    )? {
        s.bloom_intensity = e.get_float();
        if s.bloom_intensity <= 0.0 {
            return Err(conf.invalid_value_error(e));
        }
    }
    if let Some(e) = need(
        conf,
        sec,
        "bloom-multiplier",
        "specify bloom-multiplier (float)",
    )? {
        s.bloom_multiplier = e.get_float();
        if s.bloom_multiplier <= 0.0 {
            return Err(conf.invalid_value_error(e));
        }
    }
    if let Some(e) = need(conf, sec, "elasticity", "specify elasticity (float)")? {
        s.elasticity = e.get_float();
        if s.elasticity <= 0.0 {
            return Err(conf.invalid_value_error(e));
        }
    }
    if let Some(e) = need(conf, sec, "font-file", "specify font file")? {
        let canonical = std::fs::canonicalize(&e.value).map_err(|_| conf.invalid_value_error(e))?;
        s.font_file = canonical.to_string_lossy().into_owned();
        if s.font_file.is_empty() {
            return Err(conf.invalid_value_error(e));
        }
    }
    for (key, msg, set) in [
        ("font-size", "specify font size", 0),
        ("file-font-size", "specify font size", 1),
        ("dir-font-size", "specify font size", 2),
        ("user-font-size", "specify font size", 3),
    ] {
        if let Some(e) = need(conf, sec, key, msg)? {
            let v = e.get_int();
            if !(1..=100).contains(&v) {
                return Err(conf.invalid_value_error(e));
            }
            match set {
                0 => s.font_size = v,
                1 => s.filename_font_size = v,
                2 => s.dirname_font_size = v,
                _ => s.user_font_size = v,
            }
        }
    }
    if let Some(e) = need(conf, sec, "font-scale", "specify font scale")? {
        s.font_scale = e.get_float();
        s.default_font_scale = false;
        if !(0.0..=10.0).contains(&s.font_scale) {
            return Err(conf.invalid_value_error(e));
        }
        s.set_scaled_font_sizes();
    }
    if let Some(e) = need(conf, sec, "hash-seed", "specify hash seed (integer)")? {
        s.hash_seed = e.get_int();
    }
    for (key, msg) in [
        ("font-colour", "specify font colour (FFFFFF)"),
        ("background-colour", "specify background colour (FFFFFF)"),
        ("highlight-colour", "specify highlight colour (FFFFFF)"),
        ("selection-colour", "specify selection colour (FFFFFF)"),
        ("dir-colour", "specify dir colour (FFFFFF)"),
    ] {
        if let Some(e) = need(conf, sec, key, msg)? {
            let c = colour(conf, e)?;
            match key {
                "font-colour" => s.font_colour = c,
                "background-colour" => s.background_colour = c,
                "highlight-colour" => s.highlight_colour = c,
                "selection-colour" => s.selection_colour = c,
                _ => s.dir_colour = c,
            }
        }
    }
    if let Some(e) = need(
        conf,
        sec,
        "background-image",
        "specify background image (image path)",
    )? {
        s.background_image = e.value.clone();
    }
    if let Some(e) = need(conf, sec, "title", "specify title")? {
        s.title = e.value.clone();
    }
    if let Some(e) = need(conf, sec, "logo", "specify logo (image path)")? {
        s.logo = e.value.clone();
    }
    if let Some(e) = need(conf, sec, "logo-offset", "specify logo-offset (XxY)")? {
        match parse_rectangle(&e.value) {
            Some((x, y)) => s.logo_offset = [x as f32, y as f32],
            None => return Err(conf.invalid_value_error(e)),
        }
    }
    if let Some(e) = need(
        conf,
        sec,
        "seconds-per-day",
        "specify seconds-per-day (seconds)",
    )? {
        let spd = e.get_float();
        if spd <= 0.0 {
            return Err(conf.invalid_value_error(e));
        }
        s.days_per_second = (1.0f64 / spd as f64) as f32;
    }
    if let Some(e) = need(
        conf,
        sec,
        "auto-skip-seconds",
        "specify auto-skip-seconds (seconds)",
    )? {
        s.auto_skip_seconds = e.get_float();
        if s.auto_skip_seconds <= 0.0 {
            return Err(conf.invalid_value_error(e));
        }
    }
    for (key, msg, at_end) in [
        ("file-idle-time", "specify file-idle-time (seconds)", false),
        (
            "file-idle-time-at-end",
            "specify file-idle-time-at-end (seconds)",
            true,
        ),
    ] {
        if let Some(e) = need(conf, sec, key, msg)? {
            let v = atoi(e.value.as_bytes()) as f32;
            if v < 0.0 || (v == 0.0 && !e.value.starts_with('0')) {
                return Err(conf.invalid_value_error(e));
            }
            if at_end {
                s.file_idle_time_at_end = v;
            } else {
                s.file_idle_time = v;
            }
        }
    }
    // config-file only: `user-idle-time` is not in the CLI option table
    if let Some(e) = need(
        conf,
        sec,
        "user-idle-time",
        "specify user-idle-time (seconds)",
    )? {
        s.user_idle_time = e.get_float();
        if s.user_idle_time < 0.0 {
            return Err(conf.invalid_value_error(e));
        }
    }
    if let Some(e) = need(conf, sec, "time-scale", "specify time-scale (scale)")? {
        s.time_scale = e.get_float();
        if s.time_scale <= 0.0 || s.time_scale > 4.0 {
            return Err(conf.entry_error(e, "time-scale outside of range 0.0 - 4.0"));
        }
    }
    if let Some(e) = need(
        conf,
        sec,
        "start-date",
        "specify start-date (YYYY-MM-DD hh:mm:ss)",
    )? {
        match parse_date_time(e.value.as_bytes()) {
            Some(ts) => {
                s.start_timestamp = ts;
                s.start_date = local_date(ts);
            }
            None => return Err(conf.invalid_value_error(e)),
        }
    }
    if let Some(e) = need(
        conf,
        sec,
        "stop-date",
        "specify stop-date (YYYY-MM-DD hh:mm:ss)",
    )? {
        match parse_date_time(e.value.as_bytes()) {
            Some(ts) => {
                s.stop_timestamp = ts;
                // local time-of-day other than midnight rounds the date up a day
                use chrono::{TimeZone, Timelike};
                let nonzero = chrono::Local
                    .timestamp_opt(ts, 0)
                    .single()
                    .is_some_and(|t| t.hour() > 0 || t.minute() > 0 || t.second() > 0);
                s.stop_date = local_date(if nonzero { ts + 60 * 60 * 24 } else { ts });
            }
            None => return Err(conf.invalid_value_error(e)),
        }
    }
    if let Some(e) = need(
        conf,
        sec,
        "start-position",
        "specify start-position (float,random)",
    )? {
        if e.value == "random" {
            s.start_position_random = true;
        } else {
            s.start_position = e.get_float();
            if s.start_position <= 0.0 || s.start_position >= 1.0 {
                return Err(conf.entry_error(
                    e,
                    "start-position outside of range 0.0 - 1.0 (non-inclusive)",
                ));
            }
        }
    }
    if let Some(e) = need(conf, sec, "stop-position", "specify stop-position (float)")? {
        s.stop_position = e.get_float();
        if s.stop_position <= 0.0 || s.stop_position > 1.0 {
            return Err(conf.entry_error(e, "stop-position outside of range 0.0 - 1.0 (inclusive)"));
        }
    }
    if let Some(e) = need(conf, sec, "stop-at-time", "specify stop-at-time (seconds)")? {
        s.stop_at_time = e.get_float();
        if s.stop_at_time <= 0.0 {
            return Err(conf.invalid_value_error(e));
        }
    }
    s.show_key = sec.get_bool("key");
    // `ffp` and `stop-on-idle` are `ArgType::Ignored`: stored in the ConfFile, never read here
    if sec.get_bool("realtime") {
        s.days_per_second = (1.0f64 / 86400.0) as f32;
    }
    s.no_time_travel = sec.get_bool("no-time-travel");
    s.dont_stop = sec.get_bool("dont-stop");
    s.stop_at_end = sec.get_bool("stop-at-end");
    s.fixed_user_size = sec.get_bool("fixed-user-size");
    s.author_time = sec.get_bool("author-time");

    if let Some(e) = need(conf, sec, "max-files", "specify max-files (number)")? {
        s.max_files = e.get_int();
        if s.max_files < 0 || (s.max_files == 0 && e.value != "0") {
            return Err(conf.invalid_value_error(e));
        }
    }
    if let Some(e) = need(conf, sec, "max-file-lag", "specify max-file-lag (seconds)")? {
        s.max_file_lag = e.get_float();
        if s.max_file_lag == 0.0 {
            return Err(conf.invalid_value_error(e));
        }
    }
    if let Some(e) = need(
        conf,
        sec,
        "user-friction",
        "specify user-friction (seconds)",
    )? {
        s.user_friction = e.get_float();
        if s.user_friction <= 0.0 {
            return Err(conf.invalid_value_error(e));
        }
        s.user_friction = (1.0f64 / s.user_friction as f64) as f32;
    }
    if let Some(e) = need(conf, sec, "user-scale", "specify user-scale (scale)")? {
        s.user_scale = e.get_float();
        if s.user_scale <= 0.0 || s.user_scale > 100.0 {
            return Err(conf.invalid_value_error(e));
        }
    }
    if let Some(e) = need(
        conf,
        sec,
        "max-user-speed",
        "specify max-user-speed (units)",
    )? {
        s.max_user_speed = e.get_float();
        if s.max_user_speed <= 0.0 {
            return Err(conf.invalid_value_error(e));
        }
    }
    s.highlight_all_users = sec.get_bool("highlight-users") || sec.get_bool("highlight-all-users");
    s.highlight_dirs = sec.get_bool("highlight-dirs");
    if let Some(e) = need(
        conf,
        sec,
        "camera-mode",
        "specify camera-mode (overview,track)",
    )? {
        s.camera_mode = e.value.clone();
        if s.camera_mode != "overview" && s.camera_mode != "track" {
            return Err(conf.invalid_value_error(e));
        }
    }
    if let Some(e) = need(conf, sec, "padding", "specify padding (float)")? {
        s.padding = e.get_float();
        if s.padding <= 0.0 || s.padding >= 2.0 {
            return Err(conf.invalid_value_error(e));
        }
    }
    s.highlight_users = strings(conf, sec, "highlight-user")?;
    s.follow_users = strings(conf, sec, "follow-user")?;
    s.file_extensions = sec.get_bool("file-extensions");
    s.file_extension_fallback = sec.get_bool("file-extension-fallback");
    s.file_filters = filters(conf, sec, "file-filter")?;
    s.file_show_filters = filters(conf, sec, "file-show-filter")?;
    s.user_filters = filters(conf, sec, "user-filter")?;
    s.user_show_filters = filters(conf, sec, "user-show-filter")?;
    if let Some(e) = need(
        conf,
        sec,
        "dir-name-depth",
        "specify dir-name-depth (depth)",
    )? {
        s.dir_name_depth = e.get_int();
        if s.dir_name_depth <= 0 {
            return Err(conf.invalid_value_error(e));
        }
    }
    if let Some(e) = need(
        conf,
        sec,
        "dir-name-position",
        "specify dir-name-position (float)",
    )? {
        s.dir_name_position = e.get_float();
        if s.dir_name_position < 0.1 || s.dir_name_position > 1.0 {
            return Err(conf.entry_error(
                e,
                "dir-name-position outside of range 0.1 - 1.0 (inclusive)",
            ));
        }
    }

    // path: thrown with an empty file name, so no prefix
    if sec.has_value("path") {
        s.path = sec.get_string("path").to_string();
        s.default_path = false;
    }
    if s.path == "-" {
        if s.log_format.is_empty() {
            return Err(ConfError(
                "log-format required when reading from STDIN".into(),
            ));
        }
    } else if !s.path.is_empty() && s.path != "." {
        if s.path.ends_with('\\') || s.path.ends_with('/') {
            s.path.pop();
        }
        if !std::path::Path::new(&s.path).exists() {
            return Err(ConfError(format!(
                "'{}' does not appear to be a valid file or directory",
                s.path
            )));
        }
    }
    Ok(s)
}

/// Imports every `[gource]` section up front (deviation D1: a bad later section fails before
/// anything plays; C++ validates them lazily). An empty `[gource]` section is added when the
/// file has none, exactly as `importGourceSettings` does, so `--save-config` always writes it.
pub fn import_all_gource_settings(conf: &mut ConfFile) -> Result<Vec<GourceSettings>, ConfError> {
    if conf.count_section("gource") == 0 {
        conf.add_section("gource");
    }
    conf.sections("gource")
        .iter()
        .map(|sec| import_gource_settings(conf, sec))
        .collect()
}

/// How the hidden `--prng` / `--rand-script` options select the generator (ticket #22).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrngMode {
    Default,
    Glibc,
    Park,
    Script(String),
}

/// The hidden, parse-only test controls of ticket #22. The sim lanes implement the behaviour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugOptions {
    pub dump_state: Option<String>,
    pub dump_every: u32,
    pub dump_max_frames: Option<u32>,
    pub prng: PrngMode,
    pub prng_seed: Option<u64>,
    /// `--debug-pointer-script`: scripted mouse and Tab events for the dump (oracle fork option).
    pub pointer_script: Option<String>,
    prng_flag: bool,
    script_flag: bool,
}

impl Default for DebugOptions {
    fn default() -> Self {
        DebugOptions {
            dump_state: None,
            dump_every: 1,
            dump_max_frames: None,
            prng: PrngMode::Default,
            prng_seed: None,
            pointer_script: None,
            prng_flag: false,
            script_flag: false,
        }
    }
}

impl DebugOptions {
    pub fn is_debug_option(name: &str) -> bool {
        matches!(
            name,
            "debug-dump-state"
                | "debug-dump-every"
                | "debug-dump-max-frames"
                | "prng"
                | "prng-seed"
                | "rand-script"
                | "debug-pointer-script"
        )
    }

    /// Records one option. Error texts are Burl-only (Gource has none of these options).
    pub fn set(&mut self, name: &str, value: &str) -> Result<(), ConfError> {
        let invalid = || ConfError(format!("invalid {name} value"));
        let count = |v: &str| v.parse::<u32>().ok().filter(|n| *n > 0);
        match name {
            "debug-dump-state" if !value.is_empty() => self.dump_state = Some(value.to_string()),
            "debug-dump-every" => self.dump_every = count(value).ok_or_else(invalid)?,
            "debug-dump-max-frames" => {
                self.dump_max_frames = Some(count(value).ok_or_else(invalid)?)
            }
            "prng" => {
                self.prng = match value {
                    "default" => PrngMode::Default,
                    "glibc" => PrngMode::Glibc,
                    "park" => PrngMode::Park,
                    _ => return Err(invalid()),
                };
                self.prng_flag = true;
            }
            "debug-pointer-script" if !value.is_empty() => {
                self.pointer_script = Some(value.to_string())
            }
            "prng-seed" => self.prng_seed = Some(value.parse::<u64>().map_err(|_| invalid())?),
            "rand-script" if !value.is_empty() => {
                self.prng = PrngMode::Script(value.to_string());
                self.script_flag = true;
            }
            _ => return Err(invalid()),
        }
        if self.prng_flag && self.script_flag {
            return Err(ConfError("prng and rand-script cannot be combined".into()));
        }
        Ok(())
    }
}
