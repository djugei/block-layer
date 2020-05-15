use crate::Chunk;
use core::marker::PhantomData;

/// Not really an index, just accesses the Chunks chained.
/// Contains a pointer to the first Chunk and thats it.
///
/// This is just the "anchor" every interesting operation is implemented on the
/// Iterator.
pub struct Anchor<T> {
    start: *mut Chunk<T>,
}
// fixme: impl drop for Anchor

impl<'a, T> IntoIterator for &'a Anchor<T> {
    type Item = &'a [T];
    type IntoIter = AnchorIterator<'a, T>;

    fn into_iter(self) -> <Self as std::iter::IntoIterator>::IntoIter {
        AnchorIterator::new(self)
    }
}

pub struct AnchorIterator<'a, T> {
    // we just keep the index around for lifetime reasons
    _index: PhantomData<&'a Anchor<T>>,
    chunk: *const Chunk<T>,
}

impl<'a, T> AnchorIterator<'a, T> {
    pub fn new(index: &'a Anchor<T>) -> Self {
        Self {
            chunk: index.start,
            _index: Default::default(),
        }
    }
}

impl<'a, T> Iterator for AnchorIterator<'a, T> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        // this is safe Anchor owns the chunk
        // and  we hold a ref to it so lifetimes work out
        let chunk_ref = unsafe { self.chunk.as_ref() };
        if let Some(chunk) = chunk_ref {
            // inside a Anchor Chunks contain a pointer as their next_hint.
            self.chunk = chunk.next_hint as *const _;
            Some(chunk)
        } else {
            None
        }
    }
}

impl<'a, T> IntoIterator for &'a mut Anchor<T> {
    type Item = &'a mut [T];
    type IntoIter = AnchorIteratorMut<'a, T>;

    fn into_iter(self) -> <Self as std::iter::IntoIterator>::IntoIter {
        AnchorIteratorMut::new(self)
    }
}

pub struct AnchorIteratorMut<'a, T> {
    // we just keep the index around for lifetime reasons
    _index: PhantomData<&'a mut Anchor<T>>,
    chunk: *mut Chunk<T>,
}

impl<'a, T> AnchorIteratorMut<'a, T> {
    pub fn new(index: &'a mut Anchor<T>) -> Self {
        Self {
            chunk: index.start,
            _index: Default::default(),
        }
    }
}

impl<'a, T> Iterator for AnchorIteratorMut<'a, T> {
    type Item = &'a mut [T];
    fn next(&mut self) -> Option<&'a mut [T]> {
        // this is safe Anchor owns the chunk
        // and  we hold a ref to it so lifetimes work out
        let chunk_ref = unsafe { self.chunk.as_mut() };
        if let Some(chunk) = chunk_ref {
            // inside a Anchor Chunks contain a pointer as their next_hint.
            self.chunk = chunk.next_hint as *mut Chunk<T>;
            // we can safely pass out multiple &muts because its a different one each iteration
            // we never pass two &mut to the same chunk
            Some(chunk)
        } else {
            None
        }
    }
}
