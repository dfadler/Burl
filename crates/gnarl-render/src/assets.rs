//! Resources embedded in the binary (D9). The files under `assets/` are copies of the ones in the
//! repository's `data/` tree, kept inside this crate so that `cargo package` ships them; the
//! `assets_match_data` test fails when the two drift apart.

pub const BEAM_PNG: &[u8] = include_bytes!("../assets/beam.png");
pub const FILE_PNG: &[u8] = include_bytes!("../assets/file.png");
pub const USER_PNG: &[u8] = include_bytes!("../assets/user.png");
