#![feature(maybe_uninit_extra)]
#![feature(maybe_uninit_ref)]
#![feature(new_uninit)]
#![feature(const_generics)]
#![allow(incomplete_features)]
//! A List of singly-linked Chunks.
//!
//! Each Chunk contains multiple Elements of the stored data type.
//!
//! ## Features
//!
//! Combines the advantages and disadvantages of Vec and LinkedList:
//!
//! * deletion and insertion in the middle of the ChunkList are cheap-ish
//! * iteration does not incur cache-misses just like a Vec
//! * random access is only semi-supported (using a small index)
//!
//! Additional advantages:
//! * insertion does not re-allocate, in fact no reallocations at all.
//! * only fixed-size allocations, can be used without a real allocator.
//!
//!
//! ## Usecase
//!
//! Could be useful if you have multiple MB of Data that you need to add/delete in
//! Like when you are implementing a database for example.
//!
//! Storing the chunks in a Memory-mapped file will also be supported,
//! tough the indexing will look a bit more complicated.

mod chunk;
pub use chunk::Chunk;

pub mod index;
//pub use index::FlatIndex;
