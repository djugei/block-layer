#![feature(maybe_uninit_extra)]
#![feature(maybe_uninit_ref)]
#![feature(new_uninit)]
#![feature(option_unwrap_none)]
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

mod base_chunk;
pub use base_chunk::Chunk;

pub mod anchor;
pub mod freelist;
pub mod ptrlist;
pub mod rle;
pub mod slicelist;
pub mod sorted_list;
pub mod superblock;
