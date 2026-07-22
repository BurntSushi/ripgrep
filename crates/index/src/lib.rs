#![allow(warnings)]

pub use index::{Index, IndexBuilder, IndexDiscovery, IndexWriter};

mod index;
pub mod literal;
mod writer;
