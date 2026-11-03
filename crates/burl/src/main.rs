#![forbid(unsafe_code)]
//! Entry point: the `main.cpp` flow from argument parsing through settings import,
//! `--save-config` and `--output-custom-log`. Arguments become a `ConfFile` (`burl-conf`), which
//! is imported and validated for every `[gource]` section up front (deviation D1); there is no
//! display path yet, so a run that gets that far exits 1.

use burl_conf::args::parse_args;
use burl_conf::conffile::{ConfError, ConfFile};
use burl_conf::help::{HELP_LONG, HELP_SHORT};
use burl_conf::settings::{
    import_all_gource_settings, import_display_settings, DebugOptions, DisplaySettings,
    GourceSettings, PrngMode,
};
use burl_formats::formats::{bzr, cvs2cl, cvs_exp, git, hg, svn};
use burl_formats::{fetch_log, LogSettings, RCommit};
use burl_sim::{DumpOptions, Sim, SimSettings};
use burl_util::prng::{parse_script, PrngMode as SimPrngMode, SimRng};
use std::io::{BufRead, Write};
use std::sync::Arc;

const EXEC: &str = "burl";

/// `SDLAppQuit`: stderr `gource: <msg>` + the Try line + a blank line, exit 1.
fn quit(msg: &str) -> ! {
    let _ = std::io::stdout().flush();
    let mut err = std::io::stderr().lock();
    let _ = write!(
        err,
        "{}: {}\nTry '{}--help' for more information.\n\n",
        EXEC,
        msg,
        format_args!("{} ", EXEC)
    );
    let _ = err.flush();
    std::process::exit(1);
}

/// `SDLAppInfo`: stdout `<msg>\n`, exit 0.
fn info(msg: &str) -> ! {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", msg);
    let _ = out.flush();
    std::process::exit(0);
}

/// `GourceSettings::help`: the golden text verbatim, exit 0.
fn help(extended: bool) -> ! {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(if extended { HELP_LONG } else { HELP_SHORT }.as_bytes());
    let _ = out.flush();
    std::process::exit(0);
}

/// The command-line-section state (`GourceSettings::load_config` etc.): never stored in the
/// ConfFile, so it survives the `conf.clear()` between the two argument passes.
#[derive(Default)]
struct CommandLine {
    load_config: String,
    save_config: String,
    output_custom_filename: String,
    /// Hidden test-only controls (ticket #22): `--debug-dump-*`, `--prng`, `--rand-script`.
    debug: DebugOptions,
}

/// `commandLineOption`. The VCS printers and help exit from here, in argument order.
fn command_line_option(name: &str, value: &str, cl: &mut CommandLine) -> Result<(), ConfError> {
    match name {
        "help" => help(false),
        "extended-help" => help(true),
        _ => {}
    }
    if DebugOptions::is_debug_option(name) {
        return cl.debug.set(name, value);
    }
    if name == "load-config" && !value.is_empty() {
        cl.load_config = value.to_string();
        return Ok(());
    }
    if name == "save-config" && !value.is_empty() {
        cl.save_config = value.to_string();
        return Ok(());
    }
    let settings = LogSettings::default();
    let log_command = if name == "log-command" { value } else { "" };
    if name == "git-log-command" || log_command == "git" {
        info(&git::log_command(&settings));
    }
    if name == "cvs-exp-command" || log_command == "cvs-exp" {
        info(cvs_exp::LOG_COMMAND);
    }
    if log_command == "cvs" {
        return Err(ConfError("please use either 'cvs2cl' or 'cvs-exp'".into()));
    }
    if name == "cvs2cl-command" || log_command == "cvs2cl" {
        info(cvs2cl::LOG_COMMAND);
    }
    if name == "svn-log-command" || log_command == "svn" {
        info(&svn::log_command(&settings));
    }
    if name == "hg-log-command" || log_command == "hg" {
        info(&hg::log_command(&settings));
    }
    if name == "bzr-log-command" || log_command == "bzr" {
        info(&bzr::log_command(&settings));
    }
    if name == "output-custom-log" && !value.is_empty() {
        cl.output_custom_filename = value.to_string();
        return Ok(());
    }
    if name == "log-level" {
        // warn/debug/info/error/pedantic set the level; anything else is silently ignored
        return Ok(());
    }
    Err(ConfError(format!("invalid {name} value")))
}

/// `parseArgs` with the command-line handler bound to `cl`.
fn parse(
    arguments: &[Vec<u8>],
    conf: &mut ConfFile,
    files: Option<&mut Vec<String>>,
    cl: &mut CommandLine,
) -> Result<(), ConfError> {
    parse_args(arguments, conf, files, &mut |n, v| {
        command_line_option(n, v, cl)
    })
}

/// `.conf` (longer than 5 chars), `.cfg` or `.ini` (longer than 4): a positional that might be a config.
fn looks_like_config(file: &str) -> bool {
    (file.ends_with(".conf") && file.len() > 5)
        || ((file.ends_with(".cfg") || file.ends_with(".ini")) && file.len() > 4)
}

/// `Gource::writeCustomLog`.
fn write_custom_log(path: &str, output_file: &str, settings: LogSettings) {
    let settings = Arc::new(settings);
    let mut commitlog = match fetch_log(path, settings) {
        Ok(l) => l,
        Err(e) => {
            if !e.0.is_empty() {
                quit(&e.0);
            }
            return;
        }
    };
    let stdout = std::io::stdout();
    let mut out: Box<dyn Write> = if output_file == "-" {
        Box::new(std::io::BufWriter::new(stdout.lock()))
    } else {
        match std::fs::File::create(output_file) {
            Ok(f) => Box::new(std::io::BufWriter::new(f)),
            Err(_) => return, // fopen failure: silent, still exit 0
        }
    };
    while !commitlog.is_finished() {
        let mut commit = RCommit::default();
        if !commitlog.next_commit(&mut commit, true) {
            if !commitlog.is_seekable() {
                break;
            }
            continue;
        }
        for cf in &commit.files {
            let _ = write!(out, "{}|", commit.timestamp);
            let _ = out.write_all(&commit.username);
            let _ = writeln!(out, "|{}|{}", cf.action, cf.filename);
        }
    }
    let _ = out.flush();
}

/// The `LogSettings` the log layer needs from one section (moves the filters out).
fn log_settings(first: &mut GourceSettings) -> LogSettings {
    LogSettings {
        log_format: first.log_format.clone(),
        file_filters: std::mem::take(&mut first.file_filters),
        file_show_filters: std::mem::take(&mut first.file_show_filters),
        user_filters: std::mem::take(&mut first.user_filters),
        user_show_filters: std::mem::take(&mut first.user_show_filters),
        hash_seed: first.hash_seed,
        start_date: first.start_date.clone(),
        stop_date: first.stop_date.clone(),
        start_timestamp: first.start_timestamp,
        stop_timestamp: first.stop_timestamp,
        git_branch: first.git_branch.clone(),
        author_time: first.author_time,
        ..LogSettings::default()
    }
}

/// What the simulation reads, from one imported section. `--debug-dump-state` implies the
/// fixed-`dt` export path (the fork needs `-o /dev/null` for the same effect).
fn sim_settings(d: &DisplaySettings, g: &GourceSettings, repo_count: i32) -> SimSettings {
    SimSettings {
        display_width: d.width,
        display_height: d.height,
        output_framerate: d.output_framerate,
        exporting: true,
        stop_at_time: g.stop_at_time,
        stop_at_end: g.stop_at_end,
        stop_position: g.stop_position,
        stop_timestamp: g.stop_timestamp,
        start_position: g.start_position,
        days_per_second: g.days_per_second,
        auto_skip_seconds: g.auto_skip_seconds,
        no_time_travel: g.no_time_travel,
        loop_: g.loop_,
        file_idle_time: g.file_idle_time,
        file_idle_time_at_end: g.file_idle_time_at_end,
        user_idle_time: g.user_idle_time,
        time_scale: g.time_scale,
        max_files: g.max_files,
        max_user_speed: g.max_user_speed,
        max_file_lag: g.max_file_lag,
        user_friction: g.user_friction,
        user_scale: g.user_scale,
        elasticity: g.elasticity,
        filename_time: g.filename_time,
        hash_seed: g.hash_seed,
        camera_zoom_default: g.camera_zoom_default,
        camera_zoom_max: g.camera_zoom_max,
        padding: g.padding,
        crop_vertical: g.crop_vertical,
        crop_horizontal: g.crop_horizontal,
        disable_auto_rotate: g.disable_auto_rotate,
        track_users: g.camera_mode == "track",
        highlight_all_users: g.highlight_all_users,
        highlight_users: g.highlight_users.clone(),
        follow_users: g.follow_users.clone(),
        caption_file: g.caption_file.clone(),
        repo_count,
        path: g.path.clone(),
    }
}

/// `--debug-dump-state FILE`: load the log, run the simulation to its end and write the JSONL
/// dump (ticket #11). Errors are the message `quit` prints.
fn run_state_dump(
    display: &DisplaySettings,
    g: &mut GourceSettings,
    debug: &DebugOptions,
    dump_path: &str,
) -> Result<(), String> {
    // GourceShell::getNext, exporter case
    g.apply_shell_fixups(1, true);
    let settings = sim_settings(display, g, 1);

    let (rng, prng_name) = match &debug.prng {
        PrngMode::Script(file) => {
            let text =
                std::fs::read_to_string(file).map_err(|_| format!("could not read '{file}'"))?;
            let values = parse_script(&text)?;
            (SimRng::from_script(values)?, "script")
        }
        mode => {
            let (m, name) = match mode {
                PrngMode::Glibc => (SimPrngMode::Glibc, "glibc"),
                PrngMode::Park => (SimPrngMode::Park, "park"),
                _ => (SimPrngMode::Default, "default"),
            };
            let seed = match debug.prng_seed {
                Some(s) => {
                    Some(u32::try_from(s).map_err(|_| "invalid prng-seed value".to_string())?)
                }
                None => None,
            };
            (SimRng::new(m, seed), name)
        }
    };

    let path = g.path.clone();
    let log = fetch_log(&path, Arc::new(log_settings(g))).map_err(|e| e.0)?;
    let mut sim = Sim::new(settings, log, rng).map_err(|e| e.0)?;

    let file = std::fs::File::create(dump_path)
        .map_err(|_| format!("could not write to '{dump_path}'"))?;
    let mut out = std::io::BufWriter::new(file);
    let opts = DumpOptions {
        every: debug.dump_every,
        max_frames: debug.dump_max_frames.unwrap_or(0),
        prng: prng_name.to_string(),
        prng_seed: debug.prng_seed,
    };
    sim.run_dump(&mut out, &opts).map_err(|e| e.0)
}

fn raw_args() -> Vec<Vec<u8>> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        std::env::args_os().skip(1).map(|a| a.into_vec()).collect()
    }
    #[cfg(not(unix))]
    {
        std::env::args_os()
            .skip(1)
            .map(|a| a.to_string_lossy().into_owned().into_bytes())
            .collect()
    }
}

/// main.cpp steps 3-11 up to the point a display would open.
fn run() -> Result<(), ConfError> {
    let arguments = raw_args();
    let mut conf = ConfFile::new();
    let mut files = Vec::new();
    let mut cl = CommandLine::default();
    parse(&arguments, &mut conf, Some(&mut files), &mut cl)?;

    // a positional that loads as a ConfFile is the config (first match wins, then it is removed)
    if cl.load_config.is_empty() {
        if let Some(i) = files
            .iter()
            .position(|f| looks_like_config(f) && ConfFile::new().load(f).is_ok())
        {
            cl.load_config = files.remove(i);
        }
    }
    if !cl.load_config.is_empty() {
        conf.clear();
        conf.load(&cl.load_config)?;
        parse(&arguments, &mut conf, None, &mut cl)?; // CLI over the file
    }
    // the last positional is the path of every repo
    if let Some(path) = files.last() {
        if conf.count_section("gource") > 0 {
            for sec in conf.sections_mut("gource") {
                sec.set_entry("path", path);
            }
        } else {
            conf.set_entry("gource", "path", path);
        }
    }
    let display = import_display_settings(&conf)?;
    // D1: every [gource] section is validated now, not lazily as each repo starts
    let mut repos = import_all_gource_settings(&mut conf)?;

    if !cl.save_config.is_empty() {
        conf.save(&cl.save_config)?;
        std::process::exit(0);
    }
    let first = &mut repos[0];
    if !cl.output_custom_filename.is_empty() && !first.path.is_empty() {
        if first.path == "-" {
            // wait for the first byte (or EOF) without consuming it; the C++ polls every 100 ms
            let _ = std::io::stdin().lock().fill_buf();
        }
        let settings = log_settings(first);
        write_custom_log(&first.path, &cl.output_custom_filename, settings);
        std::process::exit(0);
    }
    if let Some(dump_path) = cl.debug.dump_state.clone() {
        if repos.len() > 1 {
            quit("--debug-dump-state supports a single [gource] section for now");
        }
        if let Err(e) = run_state_dump(&display, &mut repos[0], &cl.debug, &dump_path) {
            quit(&e);
        }
        std::process::exit(0);
    }
    // Not a C++ message: the display path does not exist yet.
    eprintln!("{}: the display path is not implemented yet", EXEC);
    std::process::exit(1);
}

fn main() {
    if let Err(e) = run() {
        quit(&e.0);
    }
}
