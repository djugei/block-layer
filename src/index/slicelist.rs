use crate::chunk::Link;
type Chunk<T> = crate::chunk::Chunk<T, usize>;
use std::mem::MaybeUninit;

pub struct SliceIter<'a, T> {
    data: &'a [MaybeUninit<Chunk<T>>],
    current: usize,
}

impl<'a, T> SliceIter<'a, T> {
    /// unsafety: make sure start is actually an initialzed chunk
    /// of the right type and only (recursively) next_hint-points to initialized chunks
    pub unsafe fn new(data: &'a [MaybeUninit<Chunk<T>>], start: usize) -> Self {
        Self {
            data,
            current: start,
        }
    }
}

impl<'a, T> Iterator for SliceIter<'a, T> {
    type Item = &'a [T];
    fn next(&mut self) -> std::option::Option<<Self as std::iter::Iterator>::Item> {
        if self.current == Link::<Chunk<u8>>::empty() {
            None
        } else {
            let data = &self.data[self.current];
            let data = unsafe { data.get_ref() };
            self.current = data.next_hint;

            Some(&data)
        }
    }
}

pub struct SliceIterMut<'a, T> {
    data: &'a mut [MaybeUninit<Chunk<T>>],
    current: usize,
}

impl<'a, T> SliceIterMut<'a, T> {
    /// unsafety: make sure start is actually an initialzed chunk
    /// of the right type and only (recursively) next_hint-points to initialized chunks
    /// and never has any loops
    pub unsafe fn new(data: &'a mut [MaybeUninit<Chunk<T>>], start: usize) -> Self {
        Self {
            data,
            current: start,
        }
    }
}

impl<'a, T> Iterator for SliceIterMut<'a, T> {
    type Item = &'a mut [T];
    fn next(&mut self) -> std::option::Option<<Self as std::iter::Iterator>::Item> {
        if self.current == Link::<Chunk<u8>>::empty() {
            None
        } else {
            let data = &mut self.data[self.current];
            let data = unsafe { data.get_mut() };
            // extending lifetime here, should be safe because we only ever access different spots
            // in the slice, as guaranteed by the unsafe new function
            let data: &mut Chunk<T> = unsafe { (data as *mut Chunk<T>).as_mut().unwrap() };
            self.current = data.next_hint;

            Some(data)
        }
    }
}
