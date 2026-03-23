pub mod core;
mod error;

pub use core::{
    find::{find_links, find_references},
    model::{LinkType, Reference},
    mv::mv,
    pathdiff::diff_paths,
    rename::rename,
};

pub use error::{MdrefError, Result};
