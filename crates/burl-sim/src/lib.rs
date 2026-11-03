#![forbid(unsafe_code)]
//! The simulation: a port of `Gource::logic` and the scene it drives (`src/gource.cpp`,
//! `dirnode.cpp`, `file.cpp`, `user.cpp`, `action.cpp`, `pawn.cpp`, `zoomcamera.cpp`), plus the
//! JSONL state dump the parity gate compares (`dump`, schema 1; see `reference/gource-cpp-dump`).
//!
//! No rendering or input. Every float expression keeps the C++ promotion and
//! narrowing (`f64` where a double literal or `PI` is involved) and the C++ evaluation order,
//! because the gate compares f32 bit patterns.

pub mod dump;
pub mod settings;
pub mod sim;

pub use dump::DumpOptions;
pub use settings::SimSettings;
pub use sim::{Sim, SimError};
