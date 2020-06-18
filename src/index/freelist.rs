use crate::chunk::Link;
use crate::index::slicelist::SliceIter;
use crate::index::slicelist::SliceIterMut;
use std::convert::TryInto;
use std::mem::MaybeUninit;

type Chunk<T> = crate::chunk::Chunk<T, usize>;

//todo: utilize slicelist for freelist
pub struct FreeList<'a, T> {
    initial: usize,
    // ok so this is kinda inaccurate, actually i want a chunk<ANY, usize> but thats not
    // expressible.
    // another option would be a union, but they don't support stuff with drop code
    // as of now, and i can't have chunk not have drop code conditionally (see comment on chunk
    // drop impl)
    chunks: &'a mut [MaybeUninit<Chunk<u8>>],
    phantom: std::marker::PhantomData<T>,
}

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
}

type EntryChunk = Chunk<Entry>;

impl EntryChunk {
    /// finds count free blocks, or however many are available
    /// will return Err((0,0)) if the chunk is empty
    /// will return coordinates _inside_ this chunk.
    pub fn find_space(&self, count: u32) -> Result<usize, (usize, u32)> {
        let mut max = (0, 0);
        for (i, e) in self.iter().enumerate() {
            // found an acceptable element
            if e.len >= count {
                return Ok(i);
            }

            // element is bigger than previous max
            if max.1 < e.len {
                max = (i, e.len);
            }
        }
        Err(max)
    }

    /// unsafety: only call thins on chunks you know have been initialized
    /// to be Chunk<Entry>
    pub unsafe fn from_u8(base: &MaybeUninit<Chunk<u8>>) -> &Self {
        let chunk = base as *const _ as *const MaybeUninit<Chunk<Entry>>;
        let chunk = chunk.as_ref().unwrap();
        let chunk = chunk.get_ref();
        chunk
    }

    /// unsafety: only call thins on chunks you know have been initialized
    /// to be Chunk<Entry>
    pub unsafe fn from_u8_mut(base: &mut MaybeUninit<Chunk<u8>>) -> &mut Self {
        let chunk = base as *mut _ as *mut MaybeUninit<Chunk<Entry>>;
        let chunk = chunk.as_mut().unwrap();
        let chunk = chunk.get_mut();
        chunk
    }
}
impl<'a, 'b, T> IntoIterator for &'b FreeList<'a, T>
where
    'b: 'a,
{
    type Item = (usize, &'a Chunk<Entry>);
    type IntoIter = SliceIter<'a, Entry>;
    fn into_iter(self) -> <Self as std::iter::IntoIterator>::IntoIter {
        // this is ok, the freelist is always in a consistent state
        unsafe { SliceIter::from_byteslice(&*self.chunks, self.initial) }
    }
}

impl<'a, T> FreeList<'a, T> {
    /// creates a new FreeList, writing its initial chunk at initial.
    /// during initialization only indices >= initial are touched
    /// so you can safely put data in front of initial
    /// and later manually mark it as used.
    pub fn new(c: &'a mut [MaybeUninit<Chunk<u8>>], initial: u32) -> Self {
        let len: u32 = c
            .len()
            .try_into()
            .expect("passed slice has more than 32bit chunks");
        let base = &mut c[initial as usize];
        // should be safe, chunk has way higher alignment than entry
        let base = unsafe {
            (base as *mut MaybeUninit<Chunk<u8>> as *mut MaybeUninit<Chunk<Entry>>).as_mut()
        }
        .unwrap();
        let base = Chunk::initialize(base);

        // write initial entries
        base.push(Entry {
            start: 0,
            len: initial,
        });
        let remain = len.saturating_sub(initial + 1);
        if remain > 0 {
            base.push(Entry {
                start: initial + 1,
                len: remain,
            });
        }

        // and thats it for initialization, other chunks are never touched.
        Self {
            initial: initial as usize,
            chunks: c,
            phantom: Default::default(),
        }
    }

    /// reads a previously created freelist,
    /// safety: make sure the list is actually been previously initialized
    /// don't just pass thing uninitialized data.
    ///
    /// also make sure the offsets are the same as previously.
    pub unsafe fn new_from(c: &'a mut [MaybeUninit<Chunk<u8>>], initial: usize) -> Self {
        Self {
            initial,
            chunks: c,
            phantom: Default::default(),
        }
    }

    /// marks a location as used, returns false if the location was already used.
    pub fn mark_used(&mut self, pos: usize) -> bool {
        println!("{}", pos);
        todo!()
    }

    /// marks a location as free
    /// only ever free locations that you yourself have
    /// previously marked as used.
    /// only ever free locations once
    ///
    /// this explicitly does not have a return value
    /// since you should already know if the position is
    /// used before calling this.
    ///
    /// will panic if trying to free something that is not marked as used.
    // fixme: move entire code into inner non-unsafe fn so unsafe is more visible
    pub unsafe fn free(&mut self, pos: u32, count: u32) {
        let mut free_chunk = None;
        let mut iter = SliceIterMut::from_byteslice(self.chunks, self.initial);
        while let Some((id, chunk)) = iter.next() {
            // generally empty chunks are forbidden
            // but its fine if its the initial chunk
            // (happens only when memory is completely exhausted)
            if let Some(Entry { start, len }) = chunk.last() {
                // have we arrived at the relevant chunk?
                if start + len >= pos {
                    free_chunk = Some((id, chunk));
                    break;
                }
            }
            free_chunk = Some((id, chunk));
        }

        let (id, chunk) = free_chunk.expect("freelist contains no chunks at all this is invalid");
        let next = iter.next();
        // part2: search inside the chunk
        let insert_pos = chunk.binary_search_by_key(&pos, |e| e.start).unwrap_err();

        // insert: 4 options
        // 1) nothing adjacent is free: add new entry
        // 2) pre is free: extend pre
        // 3) post is free: extend post
        // 4) both are free: merge all three
        // note: 3) and 4) may require access to the next chunk.
        let pre_adj = insert_pos != 0 && {
            let pre = chunk.get(insert_pos - 1).unwrap();
            pre.start + pre.len == pos
        };

        #[derive(Debug)]
        enum PostAdj {
            No,
            Same,
            Next,
        };

        let expected_start = pos + count;
        let post_adj = if insert_pos == chunk.len() {
            // we need to check the next chunk
            match &next {
                Some(next) => {
                    if let Some(Entry {
                        start: _expected_start,
                        ..
                    }) = next.1.first()
                    {
                        PostAdj::Next
                    } else {
                        PostAdj::No
                    }
                }
                None => PostAdj::No,
            }
        } else {
            match chunk.get(insert_pos) {
                Some(Entry { start, .. }) if *start == expected_start => PostAdj::Same,
                _ => PostAdj::No,
            }
        };

        // ordering will be conserved in all cases, as there is no such thing as overlapping
        // free regions
        // fixme: this code is slightly repettative, but i find it hard to cut down on it
        // for borrow-checking concerns
        match (pre_adj, post_adj) {
            (true, PostAdj::No) => {
                // just append to previous entry
                chunk[insert_pos - 1].len += count;
            }
            (false, PostAdj::Same) => {
                // just merge into next entry
                let e = &mut chunk[insert_pos];
                e.start = pos;
                e.len += count;
            }
            (false, PostAdj::Next) => {
                // merge into next
                let (_next_id, next) = next.unwrap();
                let e = next.first_mut().unwrap();
                e.start = pos;
                e.len += count;
            }
            (true, PostAdj::Same) => {
                let post_entry = chunk.remove(insert_pos).unwrap();
                // merge all into pre
                chunk[insert_pos - 1].len += count + post_entry.len;
            }
            (true, PostAdj::Next) => {
                // this case is especially important, as it is the only one that allows removal of
                // freelist-pages. not overly complicated though.
                let (next_id, next) = next.unwrap();
                let post_entry = next.remove(0).unwrap();
                let rem = if next.len() == 0 { true } else { false };

                chunk[insert_pos - 1].len += count + post_entry.len;

                if rem {
                    let next_next = next.next_hint;

                    std::ptr::drop_in_place(next as *mut _);
                    chunk.next_hint = next_next;

                    self.free(next_id as u32, 1);
                } else {
                    // it would be possible to balance if this and next are very un-equally full
                    // or merge if both are quite empty
                    // but that is not really required, so thats left for when there are
                    // benchmarks.
                }
            }
            (false, PostAdj::No) => {
                // this is the most complicated case: add a new entry, possibly allocating a chunk
                // but we can't really allocate right now since that would invalidate all
                // the work we just did.
                // we _can_ just quickly "steal" some space from the full chunk though.

                let entry = Entry {
                    start: pos,
                    len: count,
                };
                let succ = chunk.insert(insert_pos, entry);

                match succ {
                    // we are good, chunk still had space left
                    None => {}
                    // oh no, we gotta do something
                    Some(entry) => {
                        // by definition the chunk is full, so allocate one element from the last
                        // entry
                        let last = &mut chunk.last_mut().unwrap();
                        last.len -= 1;
                        let newchunk = last.start + last.len;
                        if last.len == 0 {
                            chunk.pop().unwrap();
                        }
                        let next = chunk.next_hint;
                        let newchunk_ref = &mut self.chunks[newchunk as usize];
                        let newchunk_ref = (newchunk_ref as *mut _
                            as *mut MaybeUninit<Chunk<Entry>>)
                            .as_mut()
                            .unwrap();
                        // this re-borrow is kinda hard to avoid
                        // -possilbe with split_mut- but still annoying
                        let chunk = &mut self.chunks[id];
                        let chunk = EntryChunk::from_u8_mut(chunk);
                        // split
                        let new = chunk.split(insert_pos, newchunk_ref);
                        // re-connect link
                        new.next_hint = next;
                        chunk.next_hint = newchunk as usize;

                        // insert
                        chunk.push(entry).unwrap();
                    }
                }
            }
        }
    }

    /// tries to allocate count adjacent chunks
    /// if successful returns Ok(pos) with the position of the first chunk
    ///
    /// if there is not that much adjacent free space returns
    /// Err(pos, len) with the position of the first chunk, and the len
    /// that was successfully allocated.
    /// if len != 0 you can then re-call this with the remaining chunks you need
    /// until your needs have been met.
    ///
    /// todo: add option to prefer exact match
    pub fn allocate(&mut self, count: u32) -> Result<usize, (usize, u32)> {
        // list is initialized
        use crate::index::slicelist::IterExt;
        use std::iter::repeat;
        let iter = unsafe { SliceIterMut::<Entry>::from_byteslice(self.chunks, self.initial) };
        let (chunk_id, in_chunk, free_entry) = if let Some(e) = iter
            .flat_map(|(id, slice)| repeat(id).zip(slice.iter_mut().enumerate()))
            .map(|(chunk, (in_chunk, entry))| (chunk, in_chunk, entry))
            .max_by_key_with_cutoff(|(_, _, e)| e.len, count)
        {
            e
        } else {
            return Err((0, 0));
        };

        let to_alloc = count.min(free_entry.len);

        let start = free_entry.start as usize;

        free_entry.allocate(to_alloc);

        if free_entry.len == 0 {
            // todo: can me removed by returning chunks in iterator
            // and changing the max to return a position
            let chunk = &mut self.chunks[chunk_id];
            let chunk = unsafe { EntryChunk::from_u8_mut(chunk) };
            chunk.remove(in_chunk);
        }

        if to_alloc == count {
            Ok(start)
        } else {
            Err((start, to_alloc))
        }
    }
}

#[test]
fn alloc_free() {
    fn count_free_chunks<'a, T>(l: &FreeList<'a, T>) -> usize {
        l.into_iter()
            .flat_map(|(_, e)| std::ops::Deref::deref(e))
            .map(|e| e.len as usize)
            .sum::<usize>()
    };

    fn check_disjunct<'a, T>(l: &FreeList<'a, T>) {
        let mut last_free = 0;
        let mut current = l.initial;
        while current != Link::<Chunk<u8>>::empty() {
            let chunk = &l.chunks[current];
            let chunk = unsafe { EntryChunk::from_u8(chunk) };
            for e in &chunk[..] {
                let ok = e.start >= last_free;
                if !ok {
                    dbg!(&chunk, e, last_free);
                }
                assert!(ok);
                last_free = e.start + e.len;
            }
            current = chunk.next_hint
        }
    }

    let mut allocations: Vec<Entry> = vec![];
    let n_chunks = 30_000;
    let mut base = Vec::with_capacity(n_chunks);
    unsafe { base.set_len(n_chunks) };

    let mut freelist = FreeList::<u8>::new(&mut base, 5);
    use rand;
    use rand::Rng;
    let mut rng = rand::thread_rng();

    'outer: loop {
        // maybe use an exponential distribution here
        let mut size = rng.gen_range(1, 50);
        let pre = count_free_chunks(&freelist);
        'retry: loop {
            let alloc = freelist.allocate(size);
            check_disjunct(&freelist);
            match alloc {
                Ok(pos) => {
                    allocations.push(Entry {
                        start: pos as u32,
                        len: size,
                    });
                    break 'retry;
                }
                Err((pos, len)) => {
                    if len == 0 {
                        break 'outer;
                    };
                    allocations.push(Entry {
                        start: pos as u32,
                        len,
                    });
                    size -= len;
                }
            }
        }
        let post = count_free_chunks(&freelist);
        assert_eq!(pre - post, size as usize);
    }

    // free in a random order to hit edge cases
    while allocations.len() != 0 {
        let alloc = rng.gen_range(0, allocations.len());
        let alloc = allocations.remove(alloc);
        let pre = count_free_chunks(&freelist);
        unsafe {
            freelist.free(alloc.start, alloc.len);
        }
        let post = count_free_chunks(&freelist);
        assert_eq!(pre + (alloc.len as usize), post);
        check_disjunct(&freelist);
    }
    let chunk = &freelist.chunks[freelist.initial];
    let chunk = unsafe { EntryChunk::from_u8(chunk) };
    assert_eq!(chunk.next_hint, Link::<Chunk<u8>>::empty());
    assert_eq!(chunk.len(), 2);
}
