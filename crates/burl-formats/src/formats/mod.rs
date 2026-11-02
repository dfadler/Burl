//! The nine parsers. Each file names its C++ source; regexes are verbatim.

pub mod apache;
pub mod bzr;
pub mod custom;
pub mod cvs2cl;
pub mod cvs_exp;
pub mod git;
pub mod gitraw;
pub mod hg;
pub mod svn;

/// Boilerplate shared by every parser struct: `base()` / `base_mut()`.
macro_rules! impl_base {
    ($t:ty) => {
        fn base(&self) -> &$crate::commitlog::CommitLogBase {
            &self.base
        }
        fn base_mut(&mut self) -> &mut $crate::commitlog::CommitLogBase {
            &mut self.base
        }
    };
}
pub(crate) use impl_base;
