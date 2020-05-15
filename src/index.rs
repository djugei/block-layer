use crate::chunk::Chunk;

// fixme: tests for the iterator

/*
/// holds a pointer to each individual chunk
/// allowing random access to the chunks
pub struct FlatIndex<T> {
    start: *mut Chunk<T>,
}
*/

mod anchor;
pub use anchor::Anchor;

use core::mem::MaybeUninit;

/// This is intended to be build from a mmap
/// stores the chunks in the map, without allocating externally.
/// will never delete chunks, only fully empty them
pub struct MapIndex<'a, T> {
    map: &'a mut [MaybeUninit<Chunk<T>>],
}
