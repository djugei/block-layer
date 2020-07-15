//! this is not true rle, it basically only marks spans of used or unused
use crate::superblock::Superblock;
type Chunk<T> = crate::base_chunk::Chunk<T, usize>;

#[derive(Debug, Copy, Clone)]
pub struct Entry {
    start: u32,
    len: u32,
}

impl Entry {
    /// make sure you check self.len == 0 and remove after calling this.
    fn allocate(&mut self, count: u32) {
        self.start += count;
        self.len -= count;
    }

    fn mark(&mut self, pos: u32) -> Option<Self> {
        if pos == self.start {
            self.start += 1;
            return None;
        } else if pos == self.start + self.len {
            self.len -= 1;
            return None;
        } else {
            let mut other = self.clone();

            self.len = self.start - pos;
            other.len -= self.len;
            other.len -= 1;
            other.start = self.start + self.len + 1;

            Some(other)
        }
    }
}

type EntryChunk = Chunk<Entry>;

impl EntryChunk {
    /// tries to mark a location as used.
    ///
    /// if that would cause a new entry to be added
    /// and if this chunk is full
    /// returns the new entry and the position it should have been inserted
    /// after.
    pub fn mark(&mut self, pos: u32) -> Result<(), (usize, Entry)> {
        let mut epos = None;
        for (i, e) in self.iter_mut().enumerate() {
            // maybe off by one, and it should be > instead of >=
            if e.start + e.len >= pos {
                epos = Some((i, e));
                break;
            }
        }

        let (epos, entry) = epos.unwrap();

        let add = entry.mark(pos);

        if let Some(add) = add {
            if let Err(e) = self.insert(epos + 1, add) {
                return Err((epos, e));
            }
        } else if entry.len == 0 {
            self.remove(epos);
        }

        Ok(())
    }

    /// tries to free an entry (pos+len)
    ///
    /// if that would cause the chunk to overfill returns the new entry and the position it would
    /// have been inserted at
    ///
    /// will panic on a double-free
    pub fn unmark(&mut self, e: Entry) -> Option<(usize, Entry)> {
        let insert_pos = self
            .binary_search_by_key(&e.start, |e| e.start)
            .unwrap_err();

        let pre_adj = insert_pos != 0 && {
            let pre = self.get(insert_pos - 1).unwrap();
            pre.start + pre.len == e.start
        };

        // extend existing entry
        if pre_adj {
            self.get_mut(insert_pos - 1).unwrap().len += e.len;
            None
        } else {
            if let Err(e) = self.insert(insert_pos, e) {
                Some((insert_pos, e))
            } else {
                None
            }
        }
    }
}

pub struct RleList<'s> {
    start: &'s mut (usize, usize),
    // i probably need some specialcasing in case this _is_ the freelist
    freelist: usize,
    list: &'s Superblock,
}

impl<'s> RleList<'s> {
    /// unsafe because you need to pass in valid start and freelist entries
    /// start is from locking the passed list on the correct index
    /// and freelist is the index of the freelist.
    pub unsafe fn new(
        list: &'s Superblock,
        start: &'s mut (usize, usize),
        freelist: usize,
    ) -> Self {
        Self {
            start,
            list,
            freelist,
        }
    }

    pub fn mark(&mut self, pos: u32) {
        todo!()
    }

    pub fn unmark(&mut self, e: Entry) {
        todo!()
    }

    /// actually allocated size might be smaller than requested
    /// you can call again until entry.len == 0
    pub fn alloc(&mut self, size: u32) -> Entry {
        todo!()
    }
}
