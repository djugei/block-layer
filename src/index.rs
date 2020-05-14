use crate::chunk::Chunk;

/// Not really an index, just accesses the Chunks chained.
/// Contains a pointer to the first Chunk and thats it.
///
/// This is just the "anchor" every interesting operation is implemented on the
/// Iterator.
pub struct ChainIndex<T> {
    start: *mut Chunk<T>,
}
// fixme: impl drop for ChainIndex

impl<'a, T> IntoIterator for &'a ChainIndex<T> {
    type Item = &'a [T];
    type IntoIter = ChainIndexIterator<'a, T>;

    fn into_iter(self) -> <Self as std::iter::IntoIterator>::IntoIter {
        ChainIndexIterator::new(self)
    }
}

pub struct ChainIndexIterator<'a, T> {
    // we just keep the index around for lifetime reasons
    _index: &'a ChainIndex<T>,
    chunk: *const Chunk<T>,
}

impl<'a, T> ChainIndexIterator<'a, T> {
    pub fn new(index: &'a ChainIndex<T>) -> Self {
        Self {
            chunk: index.start,
            _index: index,
        }
    }
}

impl<'a, T> Iterator for ChainIndexIterator<'a, T> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        // this is safe ChainIndex owns the chunk
        // and  we hold a ref to it so lifetimes work out
        let chunk_ref = unsafe { self.chunk.as_ref() };
        if let Some(chunk) = chunk_ref {
            // inside a ChainIndex Chunks contain a pointer as their next_hint.
            self.chunk = chunk.next_hint as *const _;
            Some(chunk)
        } else {
            None
        }
    }
}

//todo: iter_mut
//todo: iter_mut with sub-iterators (starting at current position, borrowing from iter_mut) and mutation.

// fixme: tests for the iterator

/*
/// holds a pointer to each individual chunk
/// allowing random access to the chunks
pub struct FlatIndex<T> {
    start: *mut Chunk<T>,
}
*/