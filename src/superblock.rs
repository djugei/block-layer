use core::mem::MaybeUninit;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
type Chunk<T> = crate::base_chunk::Chunk<T, usize>;

pub struct Superblock {
    c: *mut [Chunk<u8>],
}

// every call on this is either accessing a mutex or marked unsafe
unsafe impl Sync for Superblock {}

impl Superblock {
    pub fn lock(&self, pos: usize) -> Option<&mut (usize, usize)> {
        let superblock = self.c as *mut Chunk<u8> as *mut Chunk<(AtomicBool, (usize, usize))>;
        let len = unsafe { *Chunk::len_ptr(superblock) };
        if pos > len as usize {
            panic!("called lock on an out of bounds element, this should never happen. only call lock on known elements")
        }
        let entries = superblock as *mut (AtomicBool, (usize, usize));
        let entry = unsafe { entries.add(pos) };
        let entry = entry as *mut AtomicBool;
        let entry = unsafe { entry.as_ref().unwrap() };

        // todo: maybe AcqRel is enough here
        if let false = entry.compare_and_swap(false, true, Ordering::SeqCst) {
            let superblock = superblock as *mut (AtomicBool, (usize, usize));
            // this is probably ok as we would have paniced on oob when getting the lock.
            let entry = unsafe { superblock.add(pos) };
            let entry = unsafe { entry.as_mut() }.unwrap();
            Some(&mut entry.1)
        } else {
            None
        }
    }

    /// safety: only ever call this with a pos that you have previously locked through the
    /// lock() call.
    ///
    /// this is marked as unsafe so you can't safely do
    /// unlock(foregin); lock(foregin); access(foregin);
    pub unsafe fn unlock(&self, pos: usize) -> () {
        let superblock = self.c as *mut Chunk<u8> as *mut Chunk<(AtomicBool, (usize, usize))>;
        // im not 100% sure this is safe, as we are giving out &mut references
        // maybe i need to stay in pointer space until i get the atomic locked
        let lockblock = superblock.as_ref().unwrap();
        let entry = &lockblock[pos];

        // todo: maybe AcqRel is enough here
        if let true = entry.0.compare_and_swap(false, true, Ordering::SeqCst) {
            ()
        } else {
            panic!("tried to unlock an unlocked mutex");
        }
    }

    /// safety: only ever call this with a pos that you know no one else is accessing.
    /// you can ensure that for the first chunk by using the lock() and unlock() functionality
    /// of this type.
    ///
    /// only ever call this with a pos that is in bounds.
    ///
    /// don't ever call this a second time without releasing the first time.
    pub unsafe fn get_mut<T>(&self, pos: usize) -> &mut MaybeUninit<Chunk<T>> {
        let c = self.c as *mut Chunk<T> as *mut MaybeUninit<Chunk<T>>;
        c.add(pos).as_mut().unwrap()
    }

    /// safety: only ever call this with a pos that you know no one else is accessing.
    /// you can ensure that for the first chunk by using the lock() and unlock() functionality
    /// of this type.
    ///
    /// only ever call this with a pos that is in bounds.
    ///
    /// you can safely call this multiple times.
    pub unsafe fn get<T>(&self, pos: usize) -> &MaybeUninit<Chunk<T>> {
        let c = self.c as *mut Chunk<T> as *mut MaybeUninit<Chunk<T>>;
        c.add(pos).as_ref().unwrap()
    }
}
