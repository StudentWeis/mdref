pub mod core;
mod error;

#[doc(hidden)]
pub mod test_utils;

pub use core::{
    find::{find_links, find_references},
    model::{LinkType, Reference},
    mv::{mv, preview_move},
    pathdiff::diff_paths,
    progress::{NoopProgress, ProgressReporter},
    rename::rename,
};

pub use error::{MdrefError, Result};
