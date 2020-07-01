use core::marker::PhantomData;
use std::mem::MaybeUninit;

type Chunk<T> = crate::base_chunk::Chunk<T, Option<Box<()>>>;

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
    start: Option<Box<Chunk<T>>>,
}

impl<T> Anchor<T> {
    pub fn new() -> Self {
        Self { start: None }
    }

    /// creates a new Anchor containing an allocated, but empty chunk.
    pub fn new_empty() -> Self {
        let start = Box::new(Chunk::new(MaybeUninit::uninit()));
        Self { start: Some(start) }
    }

    /// The regular Iterator interface can not be implemented by
    /// AnchorIteratorMut because it needs to enforce
    /// that each item is gone before the next is returned.
    /// The lifetimes around Iterator::next() do not allow for that.
    pub fn iter_mut(&mut self) -> AnchorIteratorMut<T> {
        AnchorIteratorMut::new(self)
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
    chunk: Option<&'a Chunk<T>>,
}

impl<'a, T> AnchorIterator<'a, T> {
    pub fn new(index: &'a Anchor<T>) -> Self {
        let chunk = index.start.as_ref().map(|b| b.as_ref());
        Self {
            chunk: chunk,
            _index: Default::default(),
        }
    }
}

impl<'a, T> Iterator for AnchorIterator<'a, T> {
    type Item = &'a Chunk<T>;
    fn next(&mut self) -> Option<&'a Chunk<T>> {
        if let Some(chunk) = self.chunk {
            // inside a Anchor Chunks contain a pointer as their next_hint.
            self.chunk = chunk.next_hint.as_ref().map(|b| b.as_ref());
            Some(chunk)
        } else {
            None
        }
    }
}

#[repr(transparent)]
pub struct ChunkMut<T> {
    chunk: Chunk<T>,
}

impl<T> ChunkMut<T> {
    /// splits this chunk at the specified position
    /// allocates a new chunk
    /// puts pointer to new chunk in next_hint field of current chunk.
    pub fn split(&mut self, pos: usize) {
        let mut chunk = Box::new(MaybeUninit::uninit());
        self.chunk.split(pos, chunk.as_mut());
        // split guarantees initialization.
        let mut chunk = unsafe { chunk.assume_init() };
        // time to fix up the pointers
        std::mem::swap(&mut self.chunk.next_hint, &mut chunk.next_hint);
        self.chunk.next_hint = Some(chunk);
    }

    /// push a new element to this chunk
    /// if the current chunk it full a new chunk is created instead
    /// and the element inserted into that.
    pub fn push(&mut self, element: T) {
        if let Some(element) = self.chunk.push(element) {
            self.split(self.chunk.len() - 1);
            // this unwrap won't panic since we just split, so we can guarantee that
            // a next chunk exists.
            let next_ref = self.chunk.next_hint.as_mut().unwrap().as_mut();
            // this will only fail if one element is bigger than a whole chunk
            // which would be pointless.
            next_ref.push(element).unwrap();
        } else {
            // we are good, the first push worked
        }
    }

    pub fn has_next(&self) -> bool {
        use crate::base_chunk::Link;
        !self.chunk.next_hint.is_empty()
    }
}

impl<'a, T> From<&'a mut Chunk<T>> for &'a mut ChunkMut<T> {
    fn from(other: &'a mut Chunk<T>) -> Self {
        let ptr = other as *mut _ as *mut ChunkMut<T>;
        // this is safe, ChunkMut is transparent
        // and very much written around the idea of
        // being cast from/to Chunk
        unsafe { ptr.as_mut().unwrap() }
    }
}

pub struct AnchorIteratorMut<'a, T> {
    /// we just keep the index around for lifetime reasons
    _index: PhantomData<&'a mut Anchor<T>>,
    /// chunk is always the _current_, i.e. last returned, chunk
    /// this is different from most iterators.
    /// we need that so if the chunk is modified and split
    /// this iterator still catches the newly created chunk
    chunk: Option<&'a mut Chunk<T>>,
    first: bool,
}

impl<'a, T> AnchorIteratorMut<'a, T> {
    pub fn new(index: &'a mut Anchor<T>) -> Self {
        let chunk = index.start.as_mut().map(|b| b.as_mut());
        Self {
            chunk,
            _index: Default::default(),
            first: true,
        }
    }
}
impl<'a, T> AnchorIteratorMut<'a, T> {
    /// This method is sightly different from a regular iterators next method:
    /// it takes &'b mut self instead of &mut self.
    /// This forces the user to let go of each returned value before requesting the next.
    ///
    /// As such the following use is a compile-time error:
    /// ```compile_fail
    ///  let mut a: Anchor<u8> = Anchor::new_empty();
    ///  let mut i: AnchorIteratorMut<_> = a.iter_mut();
    ///  let n = i.next().unwrap();
    ///  n.split(0);
    ///  let n = i.next().unwrap();
    ///  n.split(0);
    ///  assert!(i.next().is_some()); //~ ERROR cannot borrow `i` as mutable more than once at a time
    ///  n.push(3);
    ///  assert!(i.next().is_none());
    /// ```
    /// Regular rust tests don't support compile_fail so this is a doc-test
    ///
    /// using it in a loop works perfectly fine, even though "for" syntax is not supported
    ///
    /// ```
    /// # use chunk_list::anchor::Anchor;
    /// let mut a: Anchor<u8> = Anchor::new_empty();
    /// let mut i = a.iter_mut();
    /// while let Some(chunk) = i.next() {
    ///     // your code here
    /// }
    /// ```
    pub fn next<'b>(&'b mut self) -> Option<&'b mut ChunkMut<T>> {
        if self.chunk.is_some() {
            if self.first {
                // don't move forwards, just return
                self.first = false;
            } else {
                // todo: this code feels overly complicated
                let mut chunk = None;
                std::mem::swap(&mut chunk, &mut self.chunk);
                let chunk = chunk.unwrap();
                let hint = chunk.next_hint.as_mut();
                let hint = hint.map(|b| b.as_mut());
                self.chunk = hint;
            }
            self.chunk.as_mut().map(|c| (*c).into())
        } else {
            None
        }
    }

    /// returns the current chunk without advancing the iterator
    pub fn get<'b>(&'b mut self) -> Option<&'b mut ChunkMut<T>> {
        self.chunk.as_mut().map(|c| (*c).into())
    }
}
// separating the non-iterator functions for clarity
impl<'a, T> AnchorIteratorMut<'a, T> {
    /// searches for needle in all the chunks past the current
    ///
    /// if there are repeats, any element might be found.
    /// this is especially relevant if there are repeats across
    /// a chunk.
    ///
    /// returns Ok(offset, pos) if an element was found
    /// returns Err(offset, pos) if no element was found
    ///
    /// number is the offset of chunks from the first one
    /// (how many times .next() was called)
    /// pos is the position inside the chunk where the needle is,
    /// or should be inserted.
    ///
    /// the search does a linear scan of the chunks first and then a binary search
    /// within the matching chunk
    ///
    /// this will be able to return a reference to the chunk directly once polonius lands
    /// not right now though
    ///
    /// since the iterator was moved to the chunk, you can get a reference from the iterators
    /// current position
    ///
    /// todo: switch to polonius asap
    /// todo: try harder to return the first match/stay in the first chunk
    pub fn search<'b>(&'b mut self, needle: &T) -> Result<(usize, usize), (usize, usize)>
    where
        T: std::cmp::Ord,
    {
        let mut past_min = false;
        let mut count = 0;
        while let Some(chunk) = self.next() {
            count += 1;
            let (first, last) = match &chunk.chunk[..] {
                [first, .., last] => (first, last),
                [first] => (first, first),
                _ => continue,
            };

            if needle >= first {
                past_min = true;
            }

            if past_min && needle <= last {
                // this is for polonius
                let chunk: &mut ChunkMut<T> = &mut *chunk;
                match chunk.chunk.binary_search(needle) {
                    Ok(pos) => return Ok((count, pos)),
                    Err(pos) => return Err((count, pos)),
                }
            }

            // last chunk, even if its not in here, it should be,
            // right past the last element.
            if !chunk.has_next() {
                return Err((count, chunk.chunk.len()));
            }
        }
        unreachable!("search should terminate within the loop");
    }
}

#[test]
fn iter() {
    let a: Anchor<u8> = Anchor::new();
    let mut i = (&a).into_iter();
    assert!(i.next().is_none());
}

#[test]
fn iter_empty() {
    let a: Anchor<u8> = Anchor::new_empty();
    let mut i: AnchorIterator<_> = (&a).into_iter();
    assert!(i.next().is_some());
}

#[test]
fn iter_mut() {
    let mut a: Anchor<u8> = Anchor::new_empty();
    let mut i: AnchorIteratorMut<_> = a.iter_mut();
    let n = i.next().unwrap();
    n.split(0);
    let n = i.next().unwrap();
    n.split(0);
    assert!(i.next().is_some());
    assert!(i.next().is_none());
}
