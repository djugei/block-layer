use crate::Chunk;
use core::marker::PhantomData;
use std::mem::MaybeUninit;

/// Not really an index, just accesses the Chunks chained.
/// Contains a pointer to the first Chunk and thats it.
///
/// This is just the "anchor" every interesting per-chunk
/// operation is implemented on the Iterator.
///
/// All operations defined directly on this that need to seek
/// always start at the front for every single operation.
///
/// Does not allocate until elements are actually pushed.
pub struct Anchor<T> {
    start: *mut Chunk<T>,
}

impl<T> Drop for Anchor<T> {
    fn drop(&mut self) {
        for chunk in self {
            // turn each reference back into a pointer
            // only visiting each reference once as per iterator protocol.
            // therefore no double-frees should happen.
            unsafe { Box::from_raw(chunk as *mut Chunk<T>) };
        }
    }
}

impl<T> Anchor<T> {
    pub fn new() -> Self {
        Self {
            start: std::ptr::null_mut(),
        }
    }

    /// creates a new Anchor containing an allocated, but empty chunk.
    pub fn new_empty() -> Self {
        let b = Box::new(Chunk::new(MaybeUninit::uninit()));
        Self {
            start: Box::into_raw(b),
        }
    }
}

impl<'a, T> IntoIterator for &'a Anchor<T> {
    type Item = &'a Chunk<T>;
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
    type Item = &'a Chunk<T>;
    fn next(&mut self) -> Option<&'a Chunk<T>> {
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
    type Item = &'a mut Chunk<T>;
    type IntoIter = AnchorIteratorMut<'a, T>;

    fn into_iter(self) -> <Self as std::iter::IntoIterator>::IntoIter {
        AnchorIteratorMut::new(self)
    }
}

#[repr(transparent)]
pub struct MutChunk<T> {
    chunk: Chunk<T>,
}

impl<T> MutChunk<T> {
    /// splits this chunk at the specified position
    /// allocates a new chunk
    /// puts pointer to new chunk in next_hint field of current chunk.
    pub fn split(&mut self, pos: usize) {
        let chunk = Box::new(MaybeUninit::uninit());
        let raw_ptr = Box::into_raw(chunk);
        let id = raw_ptr as usize;

        {
            // this is safe because no one else has a &mut to this
            let ptr_ref = unsafe { raw_ptr.as_mut() }.unwrap();
            self.chunk.split(pos, id, ptr_ref);
        }
        // notice how we don't reconstruct the box, so the value is not being dropped
        // and chunk does not contain a dangling reference.
    }
    pub fn push(&mut self, iter: &mut AnchorIteratorMut<T>, element: T) {
        if let Some(element) = self.chunk.push(element) {
            self.split(self.chunk.len() - 1);
            // this is safe, we just stored the pointer there, no other &mut to it exist.
            let nextref = unsafe { (self.chunk.next_hint as *mut Chunk<T>).as_mut() }.unwrap();
            // this will only fail if one element is bigger than a whole chunk
            // which would be pointless.
            nextref.push(element).unwrap();
        } else {
            // we are good, the first push worked
        }
    }
}

//fixme: keep _current_ not next chunk to avoid iterator invalidation
//gotta have specific logic for the first element (the unused iterator)
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
    type Item = &'a mut Chunk<T>;
    fn next(&mut self) -> Option<&'a mut Chunk<T>> {
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
