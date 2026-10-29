#![forbid(unsafe_code)]
//! Phase 1 entry point: the `main.cpp` flow up to `--output-custom-log`. Argument
//! consumption follows `SDLAppSettings::parseArgs` with the full option table so unknown
//! options and value swallowing match the C++ binary; only `--log-format`,
//! `--output-custom-log`, the `*-command` printers, `-h`/`-H` and the positional path do
//! anything. Everything else is parsed and ignored (docs/rust-rewrite/phase1.md).

use gource_conf::args::{alias, arg_type, section, ArgType, Section};
use gource_conf::help::{HELP_LONG, HELP_SHORT};
use gource_formats::formats::{bzr, cvs2cl, cvs_exp, git, hg, svn};
use gource_formats::{fetch_log, LogSettings, RCommit};
use gource_util::cstr::atoi;
use gource_util::regex::Regex;
use std::io::Write;
use std::sync::{Arc, LazyLock};

const EXEC: &str = "gource";

static VIEWPORT_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::must(r"^([0-9.]+)x([0-9.]+)(!)?$"));

/// `SDLAppQuit`: stderr `gource: <msg>` + the Try line + a blank line, exit 1.
fn quit(msg: &str) -> ! {
    let _ = std::io::stdout().flush();
    let mut err = std::io::stderr().lock();
    let _ = write!(err, "{}: {}\nTry '{}--help' for more information.\n\n", EXEC, msg, format_args!("{} ", EXEC));
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

#[derive(Default)]
struct Phase1 {
    files: Vec<Vec<u8>>,
    /// Last `--log-format` value, if the option appeared (`setEntry` replaces).
    log_format: Option<Vec<u8>>,
    output_custom_filename: Option<Vec<u8>>,
}

/// `parseViewport`: `^([0-9.]+)x([0-9.]+)(!)?$` with both ints > 0.
fn parse_viewport(value: &[u8]) -> bool {
    match VIEWPORT_REGEX.captures(value) {
        Some(m) => atoi(&m[0]) > 0 && atoi(&m[1]) > 0,
        None => false,
    }
}

/// `commandLineOption`.
fn command_line_option(name: &str, value: &[u8], p: &mut Phase1) {
    match name {
        "help" => help(false),
        "extended-help" => help(true),
        _ => {}
    }
    if (name == "load-config" || name == "save-config") && !value.is_empty() {
        // ponytail: accepted and ignored in phase 1 (ConfFile comes in phase 2)
        return;
    }
    let settings = LogSettings::default();
    let log_command: &[u8] = if name == "log-command" { value } else { b"" };
    if name == "git-log-command" || log_command == b"git" {
        info(&git::log_command(&settings));
    }
    if name == "cvs-exp-command" || log_command == b"cvs-exp" {
        info(cvs_exp::LOG_COMMAND);
    }
    if log_command == b"cvs" {
        quit("please use either 'cvs2cl' or 'cvs-exp'");
    }
    if name == "cvs2cl-command" || log_command == b"cvs2cl" {
        info(cvs2cl::LOG_COMMAND);
    }
    if name == "svn-log-command" || log_command == b"svn" {
        info(&svn::log_command(&settings));
    }
    if name == "hg-log-command" || log_command == b"hg" {
        info(&hg::log_command(&settings));
    }
    if name == "bzr-log-command" || log_command == b"bzr" {
        info(&bzr::log_command(&settings));
    }
    if name == "output-custom-log" && !value.is_empty() {
        p.output_custom_filename = Some(value.to_vec());
        return;
    }
    if name == "log-level" {
        // warn/debug/info/error/pedantic set the level; anything else is silently ignored
        return;
    }
    quit(&format!("invalid {} value", name));
}

/// `parseArgs` into the phase-1 subset of the ConfFile.
fn parse_args(arguments: &[Vec<u8>], p: &mut Phase1) {
    let mut i = 0;
    while i < arguments.len() {
        let mut args: &[u8] = &arguments[i];
        let mut is_option = false;
        while args.len() > 1 && args[0] == b'-' {
            args = &args[1..];
            is_option = true;
        }
        if args.is_empty() {
            i += 1;
            continue;
        }
        if !is_option {
            p.files.push(args.to_vec());
            i += 1;
            continue;
        }
        let name_raw = String::from_utf8_lossy(args).into_owned();
        let name = alias(&name_raw).to_string();
        // NUMBERxNUMBER is a magic alias for viewport
        if name.len() > 1 && name.contains('x') && parse_viewport(name.as_bytes()) {
            i += 1;
            continue;
        }
        let Some(ty) = arg_type(&name) else { quit(&format!("unknown option {}", name)) };
        let value: Vec<u8> = if ty == ArgType::Bool {
            b"true".to_vec()
        } else if i + 1 < arguments.len() {
            i += 1;
            arguments[i].clone()
        } else {
            Vec::new()
        };
        if section(&name) == Section::CommandLine {
            command_line_option(&name, &value, p);
            i += 1;
            continue;
        }
        if name == "log-format" {
            p.log_format = Some(value);
        }
        // every other option is stored in the ConfFile and imported in phase 2
        i += 1;
    }
}

/// The `importGourceSettings` checks for the two imported keys, with the exact messages.
fn import(p: &Phase1) -> (String, String) {
    let mut log_format = String::new();
    if let Some(v) = &p.log_format {
        if v.is_empty() {
            quit("specify log-format (format)");
        }
        log_format = String::from_utf8_lossy(v).into_owned();
        if log_format == "cvs" {
            quit("please use either 'cvs2cl' or 'cvs-exp'");
        }
        if !matches!(log_format.as_str(), "git" | "cvs-exp" | "cvs2cl" | "svn" | "custom" | "hg" | "bzr" | "apache") {
            quit("invalid 'log-format' value");
        }
    }
    // path: the last positional, default "."
    let mut path = match p.files.last() {
        Some(f) => String::from_utf8_lossy(f).into_owned(),
        None => ".".to_string(),
    };
    if path == "-" {
        if log_format.is_empty() {
            quit("log-format required when reading from STDIN");
        }
        // the C++ then polls until the first byte arrives; blocking reads do the same
    } else if !path.is_empty() && path != "." && (path.ends_with('\\') || path.ends_with('/')) {
        path.pop();
    }
    (path, log_format)
}

/// `Gource::writeCustomLog`.
fn write_custom_log(path: &str, output_file: &[u8], log_format: String) {
    let settings = Arc::new(LogSettings { log_format, ..LogSettings::default() });
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
    let mut out: Box<dyn Write> = if output_file == b"-" {
        Box::new(std::io::BufWriter::new(stdout.lock()))
    } else {
        let name = String::from_utf8_lossy(output_file).into_owned();
        match std::fs::File::create(&name) {
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

fn raw_args() -> Vec<Vec<u8>> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        std::env::args_os().skip(1).map(|a| a.into_vec()).collect()
    }
    #[cfg(not(unix))]
    {
        std::env::args_os().skip(1).map(|a| a.to_string_lossy().into_owned().into_bytes()).collect()
    }
}

fn main() {
    let arguments = raw_args();
    let mut p = Phase1::default();
    parse_args(&arguments, &mut p);
    let (path, log_format) = import(&p);
    if let Some(output) = &p.output_custom_filename {
        if !path.is_empty() {
            write_custom_log(&path, output, log_format);
            std::process::exit(0);
        }
    }
    // Not a C++ message: the display path does not exist yet.
    eprintln!("{}: phase 1 build: only --output-custom-log is implemented", EXEC);
    std::process::exit(1);
}
