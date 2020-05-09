#![feature(maybe_uninit_extra)]
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

use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;

#[cfg(target_pointer_width = "64")]
const PTR_SIZE: usize = 8;
#[cfg(target_pointer_width = "32")]
const PTR_SIZE: usize = 4;
#[cfg(target_pointer_width = "16")]
const PTR_SIZE: usize = 2;

const BUF_SIZE: usize = 4096 - 2 - PTR_SIZE;

/// a single, page-sized chunk.
/// you can use this directly, or through a ChunkIndex
/// if you need random access.
#[repr(C, align(4096))]
pub struct Chunk<T> {
    /// where the user data is actually stored
    /// 4096 - 2 - 8
    buf: [u8; BUF_SIZE],
    len: u16,
    next: Option<Box<Self>>,
    phantom: PhantomData<T>,
}

impl<T> Chunk<T> {
    /// pass in an unititialized chunk of memory
    /// get out a Chunk
    pub fn new(mut store: MaybeUninit<Self>) -> Self {
        // 1) get the offsets
        let store_ptr = store.as_mut_ptr() as *mut u8;

        let align = store_ptr as usize;
        dbg!(align, store_ptr);

        let buf_ptr = store_ptr;

        // offset to "len" field
        // this is safe because its within the allocation
        let len_ptr = unsafe { store_ptr.add(BUF_SIZE) };

        // offset to "next" field
        // again, safe because inside the same allocation
        let next_ptr = unsafe { len_ptr.add(2) };

        let buf_ptr = buf_ptr as *mut u8;
        let len_ptr = len_ptr as *mut u16;
        let next_ptr = next_ptr as *mut Option<Box<Self>>;

        dbg!(buf_ptr, len_ptr, next_ptr);

        // 3) initialize
        unsafe {
            for o in 0..BUF_SIZE {
                buf_ptr.add(o).write(0);
            }
        }
        // the alignment must always work out because we don't allow for pointer sizes < 16
        unsafe { len_ptr.write(0u16) };
        unsafe { next_ptr.write(None) };

        // buf has been zero-initialized
        // the length and the next pointer have just been initialized
        // phantom is a ZST
        // Chunk is repr(C)
        // so this transmute is safe.
        unsafe { std::mem::transmute(store) }
    }

    /// pushes a value, unless the list is full
    pub fn push(&mut self, value: T) -> Option<T> {
        let len = self.len as usize;
        let values = self.as_uninit_slice_mut();

        if let Some(place) = values.get_mut(len) {
            place.write(value);
            // increment len, now that the element is written
            self.len += 1;
            None
        } else {
            Some(value)
        }
    }

    // pops the last value
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        };

        self.len -= 1;
        let last = self.len as usize;

        let values = self.as_uninit_slice_mut();

        let mut value = MaybeUninit::uninit();
        std::mem::swap(&mut value, &mut values[last]);

        // this is safe because it contains the (initialized)
        // value from the list, we just swapped it out.
        // the list now contains the uninitialized value.
        let value = unsafe { value.assume_init() };
        Some(value)
    }

    pub fn capacity(&self) -> usize {
        self.as_uninit_slice().len()
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    //todo: del, insert

    pub fn as_uninit_slice(&self) -> &[MaybeUninit<T>] {
        // this is "safe" because we only transmute it to MaybeUninit
        // i.e. not actually doing anything.
        // u8 does not have drop.
        let (_pre, values, _post) = unsafe { self.buf.align_to() };
        values
    }
    pub fn as_uninit_slice_mut(&mut self) -> &mut [MaybeUninit<T>] {
        // this is "safe" because we only transmute it to MaybeUninit
        // i.e. not actually doing anything.
        // u8 does not have drop.
        let (_pre, values, _post) = unsafe { self.buf.align_to_mut() };
        values
    }
}

impl<T> Drop for Chunk<T> {
    // gotta drop all initialized data
    fn drop(&mut self) {
        while let Some(_) = self.pop() {}
    }
}

impl<T> Deref for Chunk<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        let base = &self.buf as *const _ as *const T;

        // safe because self.len is guaranteed to actually represent the initialized len.
        unsafe { std::slice::from_raw_parts(base, self.len as usize) }
    }
}

impl<T> DerefMut for Chunk<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let base = &mut self.buf as *mut _ as *mut T;

        // safe because self.len is guaranteed to actually represent the initialized len.
        unsafe { std::slice::from_raw_parts_mut(base, self.len as usize) }
    }
}

// todo:index

#[test]
fn sizes() {
    assert_eq!(std::mem::size_of::<Chunk<u8>>(), 4096);
}

#[test]
fn push_pop() {
    let store = Box::new(MaybeUninit::uninit());

    let mut chunk = Chunk::new(*store);
    assert_eq!(chunk.capacity(), BUF_SIZE / std::mem::size_of::<usize>());

    for i in 0usize..chunk.capacity() {
        assert_eq!(chunk.push(i), None);
    }
    assert_eq!(chunk.push(0), Some(0));

    for _ in 0..chunk.capacity() {
        assert!(chunk.pop().is_some());
    }
    assert_eq!(chunk.pop(), None);
}
