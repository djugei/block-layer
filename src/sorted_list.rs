type Chunk<T> = crate::base_chunk::Chunk<T, usize>;

pub trait OrdFn<T> {
    type O: Ord;
    fn key(t: &T) -> Self::O;
}

#[repr(transparent)]
pub struct SortedChunk<T, F>
where
    F: OrdFn<T>,
{
    chunk: Chunk<T>,
    mark: std::marker::PhantomData<F>,
}

impl<T, F> SortedChunk<T, F>
where
    F: OrdFn<T>,
{
    pub fn insert(&mut self, v: T) -> Result<&mut T, T> {
        match self.chunk.binary_search_by_key(&F::key(&v), F::key) {
            Ok(pos) | Err(pos) => self.chunk.insert(pos, v),
        }
    }
    pub fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        self.chunk.get_mut(i)
    }
}

pub struct SortedList<T, F>
where
    F: OrdFn<T>,
{
    start: usize,
    mark: std::marker::PhantomData<(T, F)>,
}

//todo: custom iterator that keeps track of chunk and in-chunk position
