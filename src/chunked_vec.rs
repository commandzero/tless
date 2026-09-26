//! Append-only indexed storage whose growth never copies existing elements.
use std::ops::{Index, IndexMut, Range};

// Power-of-two chunks keep index translation cheap and bound unused capacity.
const CHUNK_LEN: usize = 4096;

#[derive(Debug)]
pub struct ChunkedVec<T> {
    chunks: Vec<Vec<T>>,
    len: usize,
}

impl<T> ChunkedVec<T> {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            len: 0,
        }
    }

    #[inline]
    pub fn push(&mut self, value: T) {
        if self.len / CHUNK_LEN == self.chunks.len() {
            self.add_chunk();
        }
        self.chunks[self.len / CHUNK_LEN].push(value);
        self.len += 1;
    }

    #[cold]
    fn add_chunk(&mut self) {
        self.chunks.push(Vec::with_capacity(CHUNK_LEN));
    }

    pub fn len(&self) -> usize {
        self.len
    }

    #[cfg(test)]
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator {
        (0..self.len).map(|index| &self[index])
    }

    /// Visit a parsed subtree sequentially without translating every node id.
    pub fn iter_range(&self, range: Range<usize>) -> impl Iterator<Item = &T> {
        let first = range.start / CHUNK_LEN;
        self.chunks[first..range.end.div_ceil(CHUNK_LEN)]
            .iter()
            .enumerate()
            .flat_map(move |(offset, chunk)| {
                let origin = (first + offset) * CHUNK_LEN;
                chunk[range.start.saturating_sub(origin)..(range.end - origin).min(chunk.len())]
                    .iter()
            })
    }

    pub fn partition_point(&self, mut predicate: impl FnMut(&T) -> bool) -> usize {
        let (mut left, mut right) = (0, self.len);
        while left < right {
            let middle = left + (right - left) / 2;
            if predicate(&self[middle]) {
                left = middle + 1;
            } else {
                right = middle;
            }
        }
        left
    }
}

impl<T> Index<usize> for ChunkedVec<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        &self.chunks[index / CHUNK_LEN][index % CHUNK_LEN]
    }
}

impl<T> IndexMut<usize> for ChunkedVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        &mut self.chunks[index / CHUNK_LEN][index % CHUNK_LEN]
    }
}
