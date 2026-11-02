//! The option tables from `GourceSettings::GourceSettings()` (gource_settings.cpp:219-345),
//! transcribed verbatim. `parseArgs` (core/settings.cpp:178) consults them in this order:
//! alias translation, magic `-WxH` viewport, type lookup (unknown -> error), value
//! consumption (bools take none, everything else swallows the next token), section routing.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgType {
    Bool,
    Int,
    Float,
    Str,
    MultiValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Gource,
    Display,
    CommandLine,
}

pub const DEFAULT_SECTION: Section = Section::Gource;

pub const ARG_ALIASES: &[(&str, &str)] = &[
    // display aliases (core/settings.cpp:46-49)
    ("f", "fullscreen"),
    ("w", "windowed"),
    ("o", "output-ppm-stream"),
    ("r", "output-framerate"),
    // gource aliases (gource_settings.cpp:222-236)
    ("p", "start-position"),
    ("a", "auto-skip-seconds"),
    ("s", "seconds-per-day"),
    ("t", "stop-at-time"),
    ("i", "file-idle-time"),
    ("e", "elasticity"),
    ("h", "help"),
    ("?", "help"),
    ("H", "extended-help"),
    ("b", "background-colour"),
    ("c", "time-scale"),
    ("background", "background-colour"),
    ("disable-bloom", "hide-bloom"),
    ("disable-progress", "hide-progress"),
    ("highlight-all-users", "highlight-users"),
];

/// Options routed to a section other than the default `[gource]`.
pub const CONF_SECTIONS: &[(&str, Section)] = &[
    ("help", Section::CommandLine),
    ("extended-help", Section::CommandLine),
    ("log-command", Section::CommandLine),
    ("git-log-command", Section::CommandLine),
    ("cvs-exp-command", Section::CommandLine),
    ("cvs2cl-command", Section::CommandLine),
    ("hg-log-command", Section::CommandLine),
    ("bzr-log-command", Section::CommandLine),
    ("svn-log-command", Section::CommandLine),
    ("load-config", Section::CommandLine),
    ("save-config", Section::CommandLine),
    ("output-custom-log", Section::CommandLine),
    ("log-level", Section::CommandLine),
    // display section (core/settings.cpp SDLAppSettings ctor)
    ("viewport", Section::Display),
    ("windowed", Section::Display),
    ("fullscreen", Section::Display),
    ("frameless", Section::Display),
    ("screen", Section::Display),
    ("window-position", Section::Display),
    ("multi-sampling", Section::Display),
    ("no-vsync", Section::Display),
    ("output-ppm-stream", Section::Display),
    ("output-framerate", Section::Display),
    ("transparent", Section::Display),
    ("high-dpi", Section::Display),
];

pub const ARG_TYPES: &[(&str, ArgType)] = &[
    // display options (core/settings.cpp)
    ("viewport", ArgType::Str),
    ("windowed", ArgType::Bool),
    ("fullscreen", ArgType::Bool),
    ("frameless", ArgType::Bool),
    ("screen", ArgType::Int),
    ("window-position", ArgType::Str),
    ("multi-sampling", ArgType::Bool),
    ("no-vsync", ArgType::Bool),
    ("output-ppm-stream", ArgType::Str),
    ("output-framerate", ArgType::Int),
    ("transparent", ArgType::Bool),
    ("high-dpi", ArgType::Bool),
    // gource options (gource_settings.cpp:252-345)
    ("help", ArgType::Bool),
    ("extended-help", ArgType::Bool),
    ("stop-on-idle", ArgType::Bool),
    ("stop-at-end", ArgType::Bool),
    ("dont-stop", ArgType::Bool),
    ("loop", ArgType::Bool),
    ("realtime", ArgType::Bool),
    ("no-time-travel", ArgType::Bool),
    ("colour-images", ArgType::Bool),
    ("hide-date", ArgType::Bool),
    ("hide-files", ArgType::Bool),
    ("hide-users", ArgType::Bool),
    ("hide-tree", ArgType::Bool),
    ("hide-usernames", ArgType::Bool),
    ("hide-filenames", ArgType::Bool),
    ("hide-dirnames", ArgType::Bool),
    ("hide-progress", ArgType::Bool),
    ("hide-bloom", ArgType::Bool),
    ("hide-mouse", ArgType::Bool),
    ("hide-root", ArgType::Bool),
    ("highlight-users", ArgType::Bool),
    ("highlight-dirs", ArgType::Bool),
    ("file-extensions", ArgType::Bool),
    ("file-extension-fallback", ArgType::Bool),
    ("fixed-user-size", ArgType::Bool),
    ("author-time", ArgType::Bool),
    ("key", ArgType::Bool),
    ("ffp", ArgType::Bool),
    ("disable-auto-rotate", ArgType::Bool),
    ("disable-auto-skip", ArgType::Bool),
    ("disable-input", ArgType::Bool),
    ("git-log-command", ArgType::Bool),
    ("cvs-exp-command", ArgType::Bool),
    ("cvs2cl-command", ArgType::Bool),
    ("svn-log-command", ArgType::Bool),
    ("hg-log-command", ArgType::Bool),
    ("bzr-log-command", ArgType::Bool),
    ("bloom-intensity", ArgType::Float),
    ("bloom-multiplier", ArgType::Float),
    ("elasticity", ArgType::Float),
    ("seconds-per-day", ArgType::Float),
    ("auto-skip-seconds", ArgType::Float),
    ("stop-at-time", ArgType::Float),
    ("max-user-speed", ArgType::Float),
    ("user-friction", ArgType::Float),
    ("padding", ArgType::Float),
    ("time-scale", ArgType::Float),
    ("dir-name-position", ArgType::Float),
    ("loop-delay-seconds", ArgType::Float),
    ("max-files", ArgType::Int),
    ("font-size", ArgType::Int),
    ("font-scale", ArgType::Float),
    ("file-font-size", ArgType::Int),
    ("dir-font-size", ArgType::Int),
    ("user-font-size", ArgType::Int),
    ("hash-seed", ArgType::Int),
    ("user-filter", ArgType::MultiValue),
    ("user-show-filter", ArgType::MultiValue),
    ("file-filter", ArgType::MultiValue),
    ("file-show-filter", ArgType::MultiValue),
    ("follow-user", ArgType::MultiValue),
    ("highlight-user", ArgType::MultiValue),
    ("log-level", ArgType::Str),
    ("background-image", ArgType::Str),
    ("logo", ArgType::Str),
    ("logo-offset", ArgType::Str),
    ("log-command", ArgType::Str),
    ("load-config", ArgType::Str),
    ("save-config", ArgType::Str),
    ("output-custom-log", ArgType::Str),
    ("path", ArgType::Str),
    ("background-colour", ArgType::Str),
    ("file-idle-time", ArgType::Str),
    ("file-idle-time-at-end", ArgType::Str),
    ("user-image-dir", ArgType::Str),
    ("default-user-image", ArgType::Str),
    ("date-format", ArgType::Str),
    ("log-format", ArgType::Str),
    ("git-branch", ArgType::Str),
    ("start-position", ArgType::Str),
    ("start-date", ArgType::Str),
    ("stop-date", ArgType::Str),
    ("stop-position", ArgType::Str),
    ("crop", ArgType::Str),
    ("hide", ArgType::Str),
    ("max-file-lag", ArgType::Str),
    ("user-scale", ArgType::Str),
    ("camera-mode", ArgType::Str),
    ("title", ArgType::Str),
    ("font-file", ArgType::Str),
    ("font-colour", ArgType::Str),
    ("dir-colour", ArgType::Str),
    ("highlight-colour", ArgType::Str),
    ("selection-colour", ArgType::Str),
    ("filename-colour", ArgType::Str),
    ("filename-time", ArgType::Float),
    ("caption-file", ArgType::Str),
    ("caption-size", ArgType::Int),
    ("caption-duration", ArgType::Float),
    ("caption-colour", ArgType::Str),
    ("caption-offset", ArgType::Int),
    ("dir-name-depth", ArgType::Int),
];

pub fn alias(name: &str) -> &str {
    ARG_ALIASES
        .iter()
        .find(|(a, _)| *a == name)
        .map(|(_, full)| *full)
        .unwrap_or(name)
}

pub fn arg_type(name: &str) -> Option<ArgType> {
    ARG_TYPES.iter().find(|(n, _)| *n == name).map(|(_, t)| *t)
}

pub fn section(name: &str) -> Section {
    CONF_SECTIONS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, s)| *s)
        .unwrap_or(DEFAULT_SECTION)
}
