//! Source-derived tests for the ConfFile grammar, `parseArgs` and the typed import. The only
//! oracle goldens that exist are help, exit codes and custom-log output (covered by
//! `crates/burl/tests/golden.rs`); the expected messages here come from reading
//! gource_settings.cpp and Core's conffile.cpp/settings.cpp (pinned commit d8d880f).

use burl_conf::args::{parse_args, ArgType, Section, ARG_TYPES, CONF_SECTIONS};
use burl_conf::conffile::{atof, ConfError, ConfFile};
use burl_conf::settings::*;

fn conf(text: &str) -> ConfFile {
    let mut c = ConfFile::new();
    c.parse(text.as_bytes()).unwrap();
    c
}

fn args(a: &[&str]) -> Vec<Vec<u8>> {
    a.iter().map(|s| s.as_bytes().to_vec()).collect()
}

/// `parseArgs` into a fresh conf; command-line options are recorded as `name=value`.
fn cli(a: &[&str]) -> Result<(ConfFile, Vec<String>, Vec<String>), ConfError> {
    let mut c = ConfFile::new();
    let (mut files, mut cmds) = (Vec::new(), Vec::new());
    parse_args(&args(a), &mut c, Some(&mut files), &mut |n, v| {
        cmds.push(format!("{n}={v}"));
        Ok(())
    })?;
    Ok((c, files, cmds))
}

fn import(text: &str) -> Result<GourceSettings, ConfError> {
    let c = conf(&format!("[gource]\n{text}\n"));
    import_gource_settings(&c, c.section("gource").unwrap())
}

fn err(text: &str) -> String {
    import(text)
        .err()
        .unwrap_or_else(|| panic!("{text:?} imported"))
        .0
}

fn saved(c: &mut ConfFile) -> String {
    let p = std::env::temp_dir().join(format!(
        "burl-conf-save-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    c.save(p.to_str().unwrap()).unwrap();
    let s = std::fs::read_to_string(&p).unwrap();
    let _ = std::fs::remove_file(&p);
    s
}

#[test]
fn grammar() {
    // comments only at column 0; blank lines skipped; value trimmed at the end only
    let c = conf("# top\n[gource]\n# c\n  key =  a b \t \nempty=\nhash=#not a comment\nx=y=z\n\n[gource]\nk=1");
    let secs = c.sections("gource");
    assert_eq!(secs.len(), 2);
    assert_eq!(secs[0].get_string("key"), "a b");
    assert_eq!(secs[0].entry("empty").unwrap().value, "");
    assert_eq!(secs[0].get_string("hash"), "#not a comment");
    assert_eq!(secs[0].get_string("x"), "y=z");
    assert_eq!(secs[0].entry("key").unwrap().line, 4);
    assert_eq!(secs[1].get_string("k"), "1"); // last line without a newline
                                              // entries before any header go to the unnamed section
    assert_eq!(conf("a=b\n").section("").unwrap().get_string("a"), "b");
    assert_eq!(conf("").sections("gource").len(), 0);
}

#[test]
fn grammar_errors() {
    // a CRLF file: the "[gource]\r" header matches (trailing \s) but the "\r" blank line fails
    for (text, line) in [
        ("[gource]\n  # indented\n", 2),
        ("[gource]\r\n\r\nk=v\n", 2),
        ("[gource]\nnoequals\n", 2),
        ("[]\n", 1),
    ] {
        let mut c = ConfFile::new();
        let e = c.parse(text.as_bytes());
        assert_eq!(
            e,
            Err(ConfError(format!(", line {line}: could not parse line"))),
            "{text:?}"
        );
    }
    let mut c = ConfFile::new();
    assert_eq!(
        c.load("/nonexistent/x.conf"),
        Err(ConfError(
            "failed to open config file /nonexistent/x.conf".into()
        ))
    );
}

#[test]
fn error_prefix_uses_file_and_line() {
    let p = std::env::temp_dir().join(format!("burl-conf-prefix-{}.conf", std::process::id()));
    std::fs::write(&p, "[gource]\ntime-scale=9\n").unwrap();
    let name = p.to_str().unwrap().to_string();
    let mut c = ConfFile::new();
    c.load(&name).unwrap();
    let e = import_gource_settings(&c, c.section("gource").unwrap()).unwrap_err();
    assert_eq!(
        e.0,
        format!("{name}, line 2: time-scale outside of range 0.0 - 4.0")
    );
    // a CLI-derived entry (line 0) of a loaded config gets only the file prefix
    c.section_mut("gource")
        .unwrap()
        .set_entry("time-scale", "9");
    let e = import_gource_settings(&c, c.section("gource").unwrap()).unwrap_err();
    assert_eq!(
        e.0,
        format!("{name}: time-scale outside of range 0.0 - 4.0")
    );
    // path errors carry no prefix at all
    c.section_mut("gource").unwrap().set_entry("path", "-");
    c.section_mut("gource")
        .unwrap()
        .set_entry("time-scale", "1");
    let e = import_gource_settings(&c, c.section("gource").unwrap()).unwrap_err();
    assert_eq!(e.0, "log-format required when reading from STDIN");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn save_order_is_by_name_then_key_with_repeats_in_order() {
    let mut c = conf("[gource]\nz=1\na=2\nf=x\nf=y\n[display]\nviewport=1x1\n[gource]\nb=3\n");
    assert_eq!(
        saved(&mut c),
        "[display]\nviewport=1x1\n\n[gource]\na=2\nf=x\nf=y\nz=1\n\n[gource]\nb=3\n\n"
    );
}

#[test]
fn parse_args_rules() {
    // hyphen stripping, aliases, swallowing, magic viewport, positional files
    let (c, files, cmds) = cli(&[
        "-s",
        "-5",
        "---hide",
        "mouse",
        "-1280x720!",
        "a.log",
        "-",
        "b.log",
        "-f",
    ])
    .unwrap();
    assert_eq!(files, ["a.log", "-", "b.log"]);
    assert!(cmds.is_empty());
    let g = c.section("gource").unwrap();
    assert_eq!(g.get_string("seconds-per-day"), "-5");
    assert_eq!(g.get_string("hide"), "mouse");
    let d = c.section("display").unwrap();
    assert_eq!(d.get_string("viewport"), "1280x720!");
    assert!(d.get_bool("fullscreen"));
    // a value-taking option at the end gets an empty value
    assert_eq!(
        cli(&["--title"])
            .unwrap()
            .0
            .section("gource")
            .unwrap()
            .entry("title")
            .unwrap()
            .value,
        ""
    );
    // errors carry the name after alias translation, hyphens stripped
    assert_eq!(
        cli(&["--version"]).err().unwrap().0,
        "unknown option version"
    );
    assert_eq!(cli(&["--"]).err().unwrap().0, "unknown option -");
    assert_eq!(
        cli(&["--seconds-per-day=1"]).err().unwrap().0,
        "unknown option seconds-per-day=1"
    );
    // `0x5` fails the viewport check and falls through to option lookup
    assert_eq!(cli(&["-0x5"]).err().unwrap().0, "unknown option 0x5");
    // user-idle-time is config-file only
    assert_eq!(
        cli(&["--user-idle-time", "3"]).err().unwrap().0,
        "unknown option user-idle-time"
    );
}

#[test]
fn cli_applies_to_every_section_multi_values_append() {
    let mut c = conf("[gource]\ntitle=a\nuser-filter=x\n[gource]\ntitle=b\n[display]\nviewport=10x10\n[display]\nscreen=2\n");
    parse_args(
        &args(&["--title", "T", "--user-filter", "y", "-800x600", "-f"]),
        &mut c,
        None,
        &mut |_, _| Ok(()),
    )
    .unwrap();
    let g = c.sections("gource");
    assert!(g.iter().all(|s| s.get_string("title") == "T"));
    let f: Vec<_> = g[0]
        .entries("user-filter")
        .iter()
        .map(|e| e.value.as_str())
        .collect();
    assert_eq!(f, ["x", "y"]);
    assert_eq!(g[1].entries("user-filter").len(), 1);
    // -WxH goes to the first [display] only; an ordinary option to all of them
    let d = c.sections("display");
    assert_eq!(d[0].get_string("viewport"), "800x600");
    assert_eq!(d[1].get_string("viewport"), "");
    assert!(d[0].get_bool("fullscreen") && d[1].get_bool("fullscreen"));
}

#[test]
fn command_line_options_run_in_argument_order_and_are_never_stored() {
    let (mut c, _, cmds) = cli(&[
        "--log-format",
        "git",
        "-h",
        "--output-custom-log",
        "-",
        "--load-config",
        "x",
        "--prng",
        "glibc",
    ])
    .unwrap();
    assert_eq!(
        cmds,
        [
            "help=true",
            "output-custom-log=-",
            "load-config=x",
            "prng=glibc"
        ]
    );
    assert_eq!(saved(&mut c), "[gource]\nlog-format=git\n\n");
    // a handler error aborts the parse before later options are looked at
    let mut c = ConfFile::new();
    let r = parse_args(&args(&["-h", "--bogus"]), &mut c, None, &mut |_, _| {
        Err(ConfError("stop".into()))
    });
    assert_eq!(r, Err(ConfError("stop".into())));
}

#[test]
fn first_end_to_end_save() {
    let (mut c, _, _) = cli(&["-1280x720", "--ffp"]).unwrap();
    assert_eq!(
        saved(&mut c),
        "[display]\nviewport=1280x720\n\n[gource]\nffp=true\n\n"
    );
}

#[test]
fn ignored_options_are_stored_and_do_nothing() {
    let ignored: Vec<_> = ARG_TYPES
        .iter()
        .filter(|(_, t)| *t == ArgType::Ignored)
        .map(|(n, _)| *n)
        .collect();
    assert_eq!(ignored, ["stop-on-idle", "ffp"]);
    let base = format!("{:?}", import("").unwrap());
    for name in ignored {
        let (c, files, _) = cli(&[&format!("--{name}"), "next.log"]).unwrap();
        assert_eq!(files, ["next.log"], "{name} must not swallow a value");
        assert_eq!(c.section("gource").unwrap().get_string(name), "true");
        assert_eq!(
            format!("{:?}", import(&format!("{name}=true")).unwrap()),
            base,
            "{name}"
        );
    }
}

#[test]
fn every_option_has_an_import_arm() {
    let src = include_str!("../src/settings.rs");
    let display: Vec<_> = CONF_SECTIONS
        .iter()
        .filter(|(_, s)| *s == Section::Display)
        .map(|(n, _)| *n)
        .collect();
    for (name, ty) in ARG_TYPES {
        let command_line = CONF_SECTIONS
            .iter()
            .any(|(n, s)| n == name && *s == Section::CommandLine);
        if *ty == ArgType::Ignored
            || command_line
            || (name.starts_with("hide-") && *ty == ArgType::Bool)
        {
            continue;
        }
        // highlight-all-users is an alias, `viewport`/`windowed` are display keys
        assert!(
            src.contains(&format!("\"{name}\"")),
            "{name} has no import arm (display: {})",
            display.contains(name)
        );
    }
}

#[test]
fn defaults_match_set_gource_defaults() {
    let s = import("").unwrap();
    assert_eq!((s.path.as_str(), s.default_path), (".", true));
    assert_eq!(
        (
            s.stop_at_time,
            s.auto_skip_seconds,
            s.days_per_second,
            s.time_scale
        ),
        (-1.0, 3.0, 0.1, 1.0)
    );
    assert_eq!(
        (
            s.hash_seed,
            s.font_size,
            s.filename_font_size,
            s.caption_size
        ),
        (31, 16, 14, 16)
    );
    assert_eq!(
        (s.user_friction, s.user_idle_time, s.max_file_lag, s.padding),
        (1.0, 3.0, 5.0, 1.1)
    );
    assert_eq!((s.scaled_font_size, s.scaled_filename_font_size), (16, 14));
    assert_eq!(s.date_format, "%A, %d %B, %Y %X");
    assert_eq!(s.selection_colour, [1.0, 1.0, 0.3]);
    let d = DisplaySettings::default();
    assert_eq!(
        (d.width, d.output_framerate, d.vsync, d.screen),
        (1024, 60, true, -1)
    );
}

#[test]
fn validation_messages() {
    let cases = [
        ("seconds-per-day=0", "invalid 'seconds-per-day' value"),
        ("seconds-per-day=", "specify seconds-per-day (seconds)"),
        ("auto-skip-seconds=-1", "invalid 'auto-skip-seconds' value"),
        ("stop-at-time=0", "invalid 'stop-at-time' value"),
        (
            "start-position=1",
            "start-position outside of range 0.0 - 1.0 (non-inclusive)",
        ),
        (
            "stop-position=1.5",
            "stop-position outside of range 0.0 - 1.0 (inclusive)",
        ),
        ("time-scale=5", "time-scale outside of range 0.0 - 4.0"),
        ("elasticity=0", "invalid 'elasticity' value"),
        ("file-idle-time=abc", "invalid 'file-idle-time' value"),
        ("file-idle-time=-1", "invalid 'file-idle-time' value"),
        (
            "file-idle-time-at-end=x",
            "invalid 'file-idle-time-at-end' value",
        ),
        ("max-files=abc", "invalid 'max-files' value"),
        ("max-file-lag=0", "invalid 'max-file-lag' value"),
        ("filename-time=1.9", "filename-time must be >= 2.0"),
        ("font-size=101", "invalid 'font-size' value"),
        ("user-font-size=0", "invalid 'user-font-size' value"),
        ("font-scale=11", "invalid 'font-scale' value"),
        ("font-colour=GG0000", "invalid 'font-colour' value"),
        (
            "background-colour=FF00",
            "invalid 'background-colour' value",
        ),
        ("camera-mode=top", "invalid 'camera-mode' value"),
        ("crop=diag", "invalid 'crop' value"),
        ("padding=2", "invalid 'padding' value"),
        ("bloom-intensity=0", "invalid 'bloom-intensity' value"),
        ("loop-delay-seconds=0", "invalid 'loop-delay-seconds' value"),
        ("dir-name-depth=0", "invalid 'dir-name-depth' value"),
        (
            "dir-name-position=0.05",
            "dir-name-position outside of range 0.1 - 1.0 (inclusive)",
        ),
        ("hide=bogus", "unknown option hide bogus"),
        ("log-format=cvs", "please use either 'cvs2cl' or 'cvs-exp'"),
        ("log-format=nope", "invalid 'log-format' value"),
        ("git-branch=-x", "invalid 'git-branch' value"),
        ("git-branch=", "no value specified for 'git-branch'"),
        ("user-filter=", "specify user-filter (regex)"),
        ("user-filter=(", "invalid user-filter regular expression"),
        (
            "file-show-filter=[",
            "invalid file-show-filter regular expression",
        ),
        ("highlight-user=", "specify highlight-user (user)"),
        ("date-format=", "no value specified for 'date-format'"),
        (
            "path=/nonexistent/x",
            "'/nonexistent/x' does not appear to be a valid file or directory",
        ),
        ("path=-", "log-format required when reading from STDIN"),
        ("caption-file=/nonexistent", "caption file not found"),
        (
            "user-image-dir=/nonexistent",
            "specified user-image-dir is not a directory",
        ),
        ("font-file=/nonexistent.ttf", "invalid 'font-file' value"),
        ("logo-offset=abc", "invalid 'logo-offset' value"),
        ("start-date=garbage", "invalid 'start-date' value"),
        ("stop-date=2020-13", "invalid 'stop-date' value"),
        ("user-idle-time=-1", "invalid 'user-idle-time' value"),
    ];
    for (text, want) in cases {
        assert_eq!(err(text), want, "{text}");
    }
}

#[test]
fn value_conversions() {
    let s = import("file-idle-time=2.5\nmax-files=0\nmax-file-lag=-1\nstart-position=0.25\nfont-scale=2\nuser-friction=4\nrealtime=yes\nhide=date,,mouse,\nbackground-colour=FF8800\nfont-colour=vec3(1, 0.5, 0)\nlogo-offset=5x6\nlog-format=git\ngit-branch=feature/x_1\nhighlight-users=1\nhash-seed=7\nuser-image-dir=/\nstart-date=2021-11-01T12:00:00Z\nstop-date=2021-11-01T00:00:00Z").unwrap();
    assert_eq!(
        (
            s.file_idle_time,
            s.max_files,
            s.max_file_lag,
            s.start_position
        ),
        (2.0, 0, -1.0, 0.25)
    );
    assert_eq!((s.scaled_font_size, s.scaled_filename_font_size), (32, 28));
    assert_eq!(s.user_friction, 0.25);
    assert_eq!(s.days_per_second, (1.0f64 / 86400.0) as f32); // realtime wins over seconds-per-day
    assert!(s.hide_date && s.hide_mouse && s.hide_progress && !s.hide_users);
    assert_eq!(s.background_colour, [1.0, 136.0 / 255.0, 0.0]);
    assert_eq!(s.font_colour, [1.0, 0.5, 0.0]);
    assert_eq!(s.logo_offset, [5.0, 6.0]);
    assert_eq!(
        (
            s.log_format.as_str(),
            s.git_branch.as_str(),
            s.highlight_all_users,
            s.hash_seed
        ),
        ("git", "feature/x_1", true, 7)
    );
    assert_eq!(
        (s.start_timestamp, s.stop_timestamp),
        (1635768000, 1635724800)
    );
    assert_eq!(s.user_image_dir, "/");
    assert!(
        import("start-position=random")
            .unwrap()
            .start_position_random
    );
    // seconds-per-day is inverted in f64, disable-auto-skip loses to an explicit auto-skip
    assert_eq!(
        import("seconds-per-day=3").unwrap().days_per_second,
        (1.0f64 / 3.0) as f32
    );
    assert_eq!(
        import("disable-auto-skip=true").unwrap().auto_skip_seconds,
        -1.0
    );
    assert_eq!(
        import("disable-auto-skip=true\nauto-skip-seconds=5")
            .unwrap()
            .auto_skip_seconds,
        5.0
    );
    // bools: only the listed spellings are true
    assert!(
        import("loop=YES").unwrap().loop_
            && !import("loop=on").unwrap().loop_
            && !import("loop=").unwrap().loop_
    );
}

#[test]
fn hide_list_quirks() {
    let h = |v: &str| {
        let s = import(&format!("hide={v}")).unwrap();
        (s.hide_date, s.hide_users, s.hide_bloom)
    };
    assert_eq!(h("date,,bloom"), (true, false, true));
    assert_eq!(h(",users,"), (false, true, false));
    assert_eq!(h(","), (false, false, false));
    assert_eq!(err("hide=date, users"), "unknown option hide  users"); // not trimmed
    assert!(import("hide-root=true").unwrap().hide_root);
}

#[test]
fn colour_forms() {
    let c = |v: &str| import(&format!("dir-colour={v}")).map(|s| s.dir_colour);
    assert_eq!(c("000000").unwrap(), [0.0; 3]);
    assert_eq!(
        c("1 2  3").unwrap(),
        [1.0 / 255.0, 2.0 / 255.0, 3.0 / 255.0]
    ); // sscanf skips spaces
    assert_eq!(c("vec3(-1,2,0.5)").unwrap(), [-1.0, 2.0, 0.5]);
    assert!(c("#FF8800").is_err() && c("FF88000").is_err() && c("ffgg00").is_err());
}

#[test]
fn atof_is_c_atof() {
    for (s, v) in [
        ("12abc", 12.0),
        ("  1.5e2x", 150.0),
        ("abc", 0.0),
        (".5", 0.5),
        ("-", 0.0),
        ("1e", 1.0),
        ("-2.", -2.0),
        ("0.0", 0.0),
        ("+3", 3.0),
    ] {
        assert_eq!(atof(s), v, "{s:?}");
    }
    assert!(atof("inf").is_infinite() && atof("NaN").is_nan());
}

#[test]
fn per_repo_import_resets_everything() {
    let mut c = conf("[gource]\nuser-filter=a\nhash-seed=9\ntitle=one\nhide=files\nstop-at-time=5\n[gource]\ntitle=two\n");
    let r = import_all_gource_settings(&mut c).unwrap();
    assert_eq!(r.len(), 2);
    let (a, b) = (&r[0], &r[1]);
    assert_eq!(
        (
            a.user_filters.len(),
            a.hash_seed,
            a.hide_files,
            a.stop_at_time
        ),
        (1, 9, true, 5.0)
    );
    assert_eq!(
        (
            b.user_filters.len(),
            b.hash_seed,
            b.hide_files,
            b.stop_at_time
        ),
        (0, 31, false, -1.0)
    );
    assert_eq!(b.title, "two");
    // a conf with no [gource] section gets an empty one
    let mut empty = ConfFile::new();
    assert_eq!(import_all_gource_settings(&mut empty).unwrap().len(), 1);
    assert_eq!(empty.count_section("gource"), 1);
}

#[test]
fn deviation_d1_all_sections_validated_up_front() {
    let mut c = conf("[gource]\ntitle=ok\n[gource]\nstop-at-time=-1\n");
    assert_eq!(
        import_all_gource_settings(&mut c).unwrap_err().0,
        "invalid 'stop-at-time' value"
    );
}

#[test]
fn shell_fixups() {
    let mut s = import("").unwrap();
    s.apply_shell_fixups(1, false);
    assert_eq!((s.stop_at_time, s.stop_at_end), (-1.0, false));
    s.apply_shell_fixups(3, true);
    assert_eq!((s.stop_at_time, s.stop_at_end), (60.0, true));
    let mut s = import("stop-at-time=5\nloop=true").unwrap();
    s.apply_shell_fixups(3, true);
    assert_eq!((s.stop_at_time, s.stop_at_end), (5.0, false));
}

#[test]
fn display_import() {
    let d = |text: &str| {
        let c = conf(&format!("[display]\n{text}\n"));
        import_display_settings(&c)
    };
    let v = d("viewport=1280x720!\nscreen=2\nwindow-position=10x0\nno-vsync=1\nhigh-dpi=true\noutput-framerate=30\noutput-ppm-stream=-").unwrap();
    assert_eq!(
        (v.width, v.height, v.resizable, v.viewport_specified),
        (1280, 720, false, true)
    );
    assert_eq!(
        (
            v.screen,
            v.window_x,
            v.window_y,
            v.vsync,
            v.high_dpi,
            v.output_framerate
        ),
        (2, 10, 0, false, true, 30)
    );
    assert_eq!(v.output_ppm_filename, "-");
    let f = d("fullscreen=true").unwrap();
    assert_eq!((f.fullscreen, f.width, f.height), (true, 0, 0));
    let w = d("fullscreen=true\nwindowed=true\nframeless=true").unwrap();
    assert_eq!((w.fullscreen, w.frameless, w.width), (false, true, 1024));
    assert!(!d("fullscreen=true\nframeless=true").unwrap().frameless);
    for (text, want) in [
        ("viewport=0x5", "invalid 'viewport' value"),
        ("viewport=abc", "invalid 'viewport' value"),
        ("screen=0", "invalid 'screen' value"),
        ("window-position=1,2", "invalid 'window-position' value"),
        ("output-framerate=45", "supported framerates are 25,30,60"),
        ("output-framerate=", "specify framerate (25,30,60)"),
        (
            "output-ppm-stream=",
            "specify ppm output file or '-' for stdout",
        ),
    ] {
        assert_eq!(d(text).unwrap_err().0, want, "{text}");
    }
    // no [display] section: defaults, even if fullscreen came from elsewhere
    assert_eq!(
        import_display_settings(&ConfFile::new()).unwrap(),
        DisplaySettings::default()
    );
}

#[test]
fn sim_settings_cover_the_gate_flags() {
    let (c, _, _) = cli(&[
        "-r",
        "30",
        "-1280x720",
        "-s",
        "1",
        "-t",
        "10",
        "--hash-seed",
        "7",
        "--disable-auto-skip",
        "--disable-auto-rotate",
        "-i",
        "2",
        "--file-idle-time-at-end",
        "1",
        "-c",
        "2",
        "--hide",
        "mouse",
    ])
    .unwrap();
    let d = import_display_settings(&c).unwrap();
    let g = import_gource_settings(&c, c.section("gource").unwrap()).unwrap();
    assert_eq!(
        SimSettings::new(&d, &g),
        SimSettings {
            framerate: 30,
            width: 1280,
            height: 720,
            days_per_second: 1.0,
            stop_at_time: 10.0,
            hash_seed: 7,
            auto_skip_seconds: -1.0,
            disable_auto_rotate: true,
            file_idle_time: 2.0,
            file_idle_time_at_end: 1.0,
            user_idle_time: 3.0,
            time_scale: 2.0,
            hide_mouse: true,
        }
    );
}

#[test]
fn hidden_debug_options() {
    let mut o = DebugOptions::default();
    assert_eq!(
        (o.dump_every, o.prng.clone(), o.prng_seed),
        (1, PrngMode::Default, None)
    );
    o.set("debug-dump-state", "out.jsonl").unwrap();
    o.set("debug-dump-every", "30").unwrap();
    o.set("debug-dump-max-frames", "4000").unwrap();
    o.set("prng-seed", "42").unwrap();
    o.set("prng", "glibc").unwrap();
    assert_eq!(
        (
            o.dump_state.as_deref(),
            o.dump_every,
            o.dump_max_frames,
            o.prng_seed,
            o.prng.clone()
        ),
        (Some("out.jsonl"), 30, Some(4000), Some(42), PrngMode::Glibc)
    );
    // running the handler again (second parse pass) is harmless
    o.set("prng", "glibc").unwrap();
    assert_eq!(
        o.set("prng", "xorshift").unwrap_err().0,
        "invalid prng value"
    );
    assert_eq!(
        o.set("debug-dump-every", "0").unwrap_err().0,
        "invalid debug-dump-every value"
    );
    assert_eq!(
        o.set("rand-script", "f").unwrap_err().0,
        "prng and rand-script cannot be combined"
    );
    let mut s = DebugOptions::default();
    s.set("rand-script", "f.txt").unwrap();
    assert_eq!(s.prng, PrngMode::Script("f.txt".into()));
    assert!(s.set("prng", "park").is_err());
    // routed to the command-line section so they are never stored or saved
    for n in [
        "debug-dump-state",
        "debug-dump-every",
        "debug-dump-max-frames",
        "prng",
        "prng-seed",
        "rand-script",
    ] {
        assert!(DebugOptions::is_debug_option(n));
        assert!(CONF_SECTIONS.contains(&(n, Section::CommandLine)), "{n}");
    }
}

/// The default build uses the regex crate for user filters; look-around only compiles in the
/// `pcre2` leg (`cargo test -p burl-conf --features pcre2`).
#[test]
fn lookaround_filters_follow_the_pcre2_feature() {
    let r = import("file-filter=(?<=a)b");
    assert_eq!(r.is_ok(), cfg!(feature = "pcre2"), "{:?}", r.err());
    if !cfg!(feature = "pcre2") {
        assert_eq!(r.err().unwrap().0, "invalid file-filter regular expression");
    }
}
