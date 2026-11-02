#![forbid(unsafe_code)]
//! Settings and CLI model. Phase 1 ships only the static option tables
//! (`args`) and the help text (`help`); ConfFile and the typed settings import
//! come in phase 2.

pub mod args;
pub mod help;
