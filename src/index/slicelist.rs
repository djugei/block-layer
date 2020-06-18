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

    /// unsafety: everything new states, and the Chunk<u8> need to actually be valid
    /// Chunk<T> for each chunk of the list
    pub unsafe fn from_byteslice(data: &'a [MaybeUninit<Chunk<u8>>], start: usize) -> Self {
        let data = (data as *const [MaybeUninit<Chunk<u8>>] as *const [MaybeUninit<Chunk<T>>])
            .as_ref()
            .unwrap();
        Self {
            data,
            current: start,
        }
    }
}

impl<'a, T> Iterator for SliceIter<'a, T> {
    type Item = (usize, &'a Chunk<T>);
    fn next(&mut self) -> std::option::Option<<Self as std::iter::Iterator>::Item> {
        if self.current == Link::<Chunk<u8>>::empty() {
            None
        } else {
            let data = &self.data[self.current];
            let data = unsafe { data.get_ref() };
            let current = self.current;
            self.current = data.next_hint;

            Some((current, &data))
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
    /// also never make changes that invalidate the list, specifically don't change
    /// next_hint to an invalid value
    pub unsafe fn new(data: &'a mut [MaybeUninit<Chunk<T>>], start: usize) -> Self {
        Self {
            data,
            current: start,
        }
    }

    /// unsafety: everything new states, and the Chunk<u8> need to actually be valid
    /// Chunk<T> for each chunk of the list
    pub unsafe fn from_byteslice(data: &'a mut [MaybeUninit<Chunk<u8>>], start: usize) -> Self {
        let data = (data as *mut [MaybeUninit<Chunk<u8>>] as *mut [MaybeUninit<Chunk<T>>])
            .as_mut()
            .unwrap();
        Self {
            data,
            current: start,
        }
    }
}

impl<'a, T> Iterator for SliceIterMut<'a, T> {
    type Item = (usize, &'a mut Chunk<T>);
    fn next(&mut self) -> std::option::Option<<Self as std::iter::Iterator>::Item> {
        if self.current == Link::<Chunk<u8>>::empty() {
            None
        } else {
            let data = &mut self.data[self.current];
            let data = unsafe { data.get_mut() };
            // extending lifetime here, should be safe because we only ever access different spots
            // in the slice, as guaranteed by the unsafe new function
            let data: &mut Chunk<T> = unsafe { (data as *mut Chunk<T>).as_mut().unwrap() };
            let current = self.current;
            self.current = data.next_hint;

            Some((current, data))
        }
    }
}

pub trait IterExt: Iterator {
    /// if the iterator contains items >= cutoff: returns the first of those
    /// if all items in the iterator are < cutoff: behaves like .max_by_key()
    fn max_by_key_with_cutoff<B, F>(mut self, mut f: F, cutoff: B) -> Option<Self::Item>
    where
        B: Ord,
        F: FnMut(&Self::Item) -> B,
        Self: Sized,
    {
        let first = self.next()?;
        let mut max = (f(&first), first);

        if max.0 >= cutoff {
            return Some(max.1);
        }

        for item in self {
            let cmp = f(&item);
            if cmp > max.0 {
                max = (cmp, item);

                if max.0 >= cutoff {
                    break;
                }
            }
        }

        Some(max.1)
    }
}

impl<T> IterExt for T where T: Iterator {}

#[test]
fn max_cutoff() {
    let a = [1, 2, 4, 5];
    let b = a.iter().max_by_key_with_cutoff(|e| **e, 6);
    assert_eq!(b, Some(&5));

    let b = a.iter().max_by_key_with_cutoff(|e| **e, 2);
    assert_eq!(b, Some(&2));

    let b = a.iter().max_by_key_with_cutoff(|e| **e, 3);
    assert_eq!(b, Some(&4));

    let a = [];
    let b = a.iter().max_by_key_with_cutoff(|e| **e, 3);
    assert_eq!(b, None);
}
