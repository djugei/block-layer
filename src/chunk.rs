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
    /// Pass in an unititialized chunk of memory
    /// get out a Chunk
    pub fn new(mut store: MaybeUninit<Self>) -> Self {
        Chunk::initialize(&mut store);
        // the initialize function guarantees that it fully
        // initializes the store.
        // therefore this is safe.
        unsafe { store.assume_init() }
    }

    /// After a call to initialize the whole struct ist guaranteed to be initialized.
    /// If the passed struct was partially initialized before, drops will not be called.
    pub fn initialize(store: &mut MaybeUninit<Self>) {
        assert!(std::mem::size_of::<T>() <= BUF_SIZE);
        assert!(std::mem::align_of::<T>() <= 4096);
        // 1) get the offsets
        let store_ptr = store.as_mut_ptr() as *mut MaybeUninit<u8>;
        let buf_ptr = store_ptr;

        // offset to "len" field
        // this is safe because its within the allocation
        let len_ptr = unsafe { store_ptr.add(BUF_SIZE) };

        // offset to "next" field
        // again, safe because inside the same allocation
        let next_ptr = unsafe { len_ptr.add(2) };

        // 2) turn into the right pointer types
        let buf_ptr = buf_ptr as *mut u8;
        let len_ptr = len_ptr as *mut u16;
        let next_ptr = next_ptr as *mut Option<Box<Self>>;

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
        // so things are correctly initialized now and we are done.
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

    /// pops the last value
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

    /// total (not remaining) capacity in this chunk
    pub fn capacity(&self) -> usize {
        self.as_uninit_slice().len()
    }

    /// number of elements in this chunk
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// inserts element at index, shifting all following elements up by one.
    /// if there is not enough space in this chunk the element is returned
    pub fn insert(&mut self, index: usize, element: T) -> Option<T> {
        let len = self.len as usize;
        let index_in_bounds = index <= len;
        let has_space = len < self.capacity();
        if index_in_bounds && has_space {
            let values = self.as_uninit_slice_mut();
            let insert_index = &mut values[index] as *mut MaybeUninit<T>;
            // this is safe because the pointer is allowed to go one past
            // in which case this will copy 0 elements
            let copy_target = unsafe { insert_index.add(1) };
            let remainder = len - index;

            // this is safe: we just checked the capacity is enough to fit one more
            // element, we are just shifting everything up by one
            unsafe { std::ptr::copy(insert_index, copy_target, remainder) }

            // we made space at the index, time to put in the new element
            values[index].write(element);

            self.len += 1;
            None
        } else {
            Some(element)
        }
    }

    /// removes and returns element at indxe
    /// if index is out of bounds, returns None
    pub fn remove(&mut self, index: usize) -> Option<T> {
        let len = self.len() as usize;
        if index < len {
            let mut val = MaybeUninit::uninit();
            let values = self.as_uninit_slice_mut();

            std::mem::swap(&mut val, &mut values[index]);
            // we checked that index is < len, so values[index] is initialized
            // we swapped the initialized value out into val
            // so now val is initialized and values[index] is not.
            let val = unsafe { val.assume_init() };

            // time to fix up the values
            let copy_target = &mut values[index] as *mut MaybeUninit<T>;
            // this is safe because the pointer is allowed to go one past
            // in which case this will copy 0 elements
            let copy_source = unsafe { copy_target.add(1) };

            // we start at index+1
            let remainder = len - (index + 1);

            // this is safe, we stay within bounds and are just shrinking
            unsafe { std::ptr::copy(copy_source, copy_target, remainder) };

            self.len -= 1;

            Some(val)
        } else {
            None
        }
    }

    //todo: del, split

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

    /// Split self at index.
    /// Everything < index stays in self, everything >= goes into other.
    /// Other will be overwritten and fully initialized.
    /// If other was partially initialized before those parts will be overwritten,
    /// not dropped
    /// if index > self.len() then this panics
    /// returns a reference other.
    pub fn split(&mut self, mut other: Box<MaybeUninit<Self>>, index: usize) -> &mut Self {
        Self::initialize(&mut *other);
        // safe because initialize guarantees initialization
        let mut other: Box<Self> = unsafe { other.assume_init() };
        let own = self.as_uninit_slice();
        let theirs = other.as_uninit_slice_mut();
        let source = &own[index..self.len()];
        let target = &mut theirs[0..source.len()];

        assert_eq!(source.len(), target.len());

        // what we are basically trying to do:
        // target.copy_from_slice(source);
        // but MaybeUninit does not implement Copy, even though it should
        let len = source.len();
        let source = source.as_ptr();
        let target = target.as_mut_ptr();
        // this is ok, we checked the lengths and everything
        unsafe { source.copy_to_nonoverlapping(target, len) };
        self.len -= len as u16;
        other.len = len as u16;

        // fix up the pointers:
        // we need to point to other, other needs to point to our next
        std::mem::swap(&mut self.next, &mut other.next);

        self.next = Some(other);
        self.next.as_mut().unwrap()
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

impl<T: std::fmt::Debug> std::fmt::Debug for Chunk<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let slice: &[T] = self;
        f.debug_list().entries(slice).finish()
    }
}

// todo:chunk-index

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

#[test]
fn insert_remove() {
    let store = Box::new(MaybeUninit::uninit());

    let mut chunk = Chunk::new(*store);

    for i in 0u128..(chunk.capacity() - 1) as u128 {
        assert_eq!(chunk.push(i), None);
    }

    assert_eq!(chunk.insert(3, 666u128), None);
    assert_eq!(chunk.insert(3, 666u128), Some(666));
    assert_eq!(chunk[3], 666u128);

    assert_eq!(chunk.remove(3), Some(666));

    // extreme cases

    let last = chunk.len();
    dbg!(chunk.capacity(), last, chunk.len(), chunk.len);
    assert_eq!(chunk.insert(last, 666), None);
    assert_eq!(chunk[last], 666u128);
    assert_eq!(chunk.remove(last), Some(666));

    //todo: check extremes (push/op at end)
}

#[test]
fn split() {
    let store = Box::new(MaybeUninit::uninit());
    let mut chunk: Chunk<u128> = Chunk::new(*store);
    let capacity = chunk.capacity();

    // only test a few because miri isn't very fast
    for s in (0..capacity)
        .step_by(32)
        .chain(std::iter::once(capacity - 1))
    {
        let store = Box::new(MaybeUninit::uninit());
        let mut chunk = Chunk::new(*store);
        for i in 0u128..(chunk.capacity()) as u128 {
            assert_eq!(chunk.push(i), None);
        }
        let store = Box::new(MaybeUninit::uninit());
        let new = chunk.split(store, s);

        assert_eq!(new.len() + chunk.len(), capacity);
        assert_eq!(chunk.len(), s);
    }
    // check that splits at 0 for 0-sized chunks also work.
    let store = Box::new(MaybeUninit::uninit());
    let new = chunk.split(store, 0);
    assert_eq!(new.len(), chunk.len());
    assert_eq!(chunk.len(), 0);
}

#[test]
#[should_panic]
fn split_oob() {
    let store = Box::new(MaybeUninit::uninit());
    let mut chunk: Chunk<u128> = Chunk::new(*store);

    let store = Box::new(MaybeUninit::uninit());
    let _new = chunk.split(store, 1);
}
