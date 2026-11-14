#![forbid(unsafe_code)]
//! Settings and CLI model: the static option tables (`args`), the raw `ConfFile` that is the
//! seam between the CLI/config-file parsers and the typed settings (`conffile`), the typed
//! import with Gource's validation messages (`settings`), and the help text (`help`).
//! Nothing here exits or prints; errors come back as `ConfError` and only the binary quits.

pub mod args;
pub mod conffile;
pub mod help;
pub mod settings;
