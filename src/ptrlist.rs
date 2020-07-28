use crate::base_chunk::Link;
type Chunk<T> = crate::base_chunk::Chunk<T, usize>;
use core::marker::PhantomData;

#[derive(Clone, Copy)]
pub struct Cursor<'a, T> {
    data: *const Chunk<T>,
    current: usize,
    phantom: PhantomData<&'a T>,
}

impl<'a, T> Cursor<'a, T> {
    /// unsafety: make sure start is actually an initialized chunk
    /// of the right type and only (recursively) next_hint-points to initialized chunks
    /// and the Chunk<u8> need to actually be valid
    /// Chunk<T> for each chunk of the list
    pub unsafe fn new(data: *const [Chunk<u8>], start: usize) -> Self {
        Self {
            data: data as *const _,
            current: start,
            phantom: PhantomData::default(),
        }
    }
}

impl<'a, T> Iterator for Cursor<'a, T> {
    type Item = (usize, &'a Chunk<T>);
    fn next(&mut self) -> std::option::Option<<Self as std::iter::Iterator>::Item> {
        if self.current == Link::<Chunk<u8>>::empty() {
            None
        } else {
            // ok cause new guarantees validity
            let data = unsafe { self.data.add(self.current) };
            let data = unsafe { data.as_ref() }.unwrap();
            let current = self.current;
            self.current = data.next_hint;

            Some((current, data))
        }
    }
}

pub struct CursorMut<'a, T> {
    data: *mut Chunk<T>,
    current: usize,
    phantom: PhantomData<&'a mut T>,
}

impl<'a, T> CursorMut<'a, T> {
    /// unsafety: make sure start is actually an initialized chunk
    /// of the right type and only (recursively) next_hint-points to initialized chunks
    /// and the Chunk<u8> need to actually be valid
    /// Chunk<T> for each chunk of the list
    /// also only ever create one CursorMut from the same start.
    /// while a CursorMut exists don't create a Cursor.
    /// If you crate multiple CursorMut with the same or overlapping datas
    /// make sure that only disjunct chunks are linked.
    /// i.e. ensure rusts aliasing rules are satisfied.
    pub unsafe fn new(data: *mut [Chunk<u8>], start: usize) -> Self {
        Self {
            data: data as *mut _,
            current: start,
            phantom: PhantomData::default(),
        }
    }
    /// Creates a "clone" of this Cursor, allowing you to move forward
    /// with the return value of this function
    /// and then snap back to where you called it.
    pub fn reborrow<'b>(&'a mut self) -> CursorMut<'b, T>
    where
        'a: 'b,
    {
        CursorMut {
            current: self.current,
            data: self.data,
            phantom: PhantomData::default(),
        }
    }
}

impl<'a, T> Iterator for CursorMut<'a, T> {
    type Item = (usize, &'a mut Chunk<T>);
    fn next(&mut self) -> std::option::Option<<Self as std::iter::Iterator>::Item> {
        if self.current == Link::<Chunk<u8>>::empty() {
            None
        } else {
            // ok cause new guarantees validity
            let data = unsafe { self.data.add(self.current) };
            let data = unsafe { data.as_mut() }.unwrap();
            let current = self.current;
            self.current = data.next_hint;

            Some((current, data))
        }
    }
}
