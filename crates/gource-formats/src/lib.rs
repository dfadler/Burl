#![forbid(unsafe_code)]
//! Log readers (`SeekLog` / `StreamLog`), the commit model, the nine parsers and the
//! format detection of `RLogMill::fetchLog`. Semantics follow log-formats.md; cites there.

pub mod commit;
pub mod commitlog;
pub mod formats;
pub mod logmill;
pub mod seeklog;
pub mod settings;
mod xml;

pub use commit::{RCommit, RCommitFile};
pub use commitlog::{CommitLog, CommitLogBase};
pub use logmill::{fetch_log, LogMillError};
pub use seeklog::{SeekLog, SeekLogError, StreamLog};
pub use settings::LogSettings;
