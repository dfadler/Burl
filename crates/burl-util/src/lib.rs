#![forbid(unsafe_code)]
//! Pure helpers shared by every other crate. Ports of `src/core/{regex,stringhash,vectors,bounds,quadtree,
//! utf8,settings(parseDateTime),timezone}` with the C++ quirks preserved; see each module.

pub mod bounds;
pub mod cstr;
pub mod log;
pub mod math;
pub mod prng;
pub mod quadtree;
pub mod regex;
pub mod strhash;
pub mod timeparse;
pub mod utf8;
