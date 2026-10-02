#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

extern crate alloc;

mod answer_table;
mod macroset;
mod microset;

pub mod incremental;
pub mod static_tree;

pub use incremental::IncrementalTreeSetUnion;
pub use static_tree::StaticTreeSetUnion;
