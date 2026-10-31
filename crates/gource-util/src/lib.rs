#![forbid(unsafe_code)]
//! Pure helpers shared by every other crate. Ports of `src/core/{regex,stringhash,vectors,
//! utf8,settings(parseDateTime),timezone}` with the C++ quirks preserved; see each module.

pub mod cstr;
pub mod math;
pub mod regex;
pub mod strhash;
pub mod timeparse;
pub mod utf8;
