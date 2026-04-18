pub mod core;
mod error;

#[doc(hidden)]
pub mod test_utils;

pub use core::{
    find::{find_links, find_references, find_references_with_progress},
    model::{LinkType, Reference},
    mv::{mv, mv_with_progress},
    pathdiff::diff_paths,
    rename::{rename, rename_with_progress},
};

pub use error::{MdrefError, Result};
