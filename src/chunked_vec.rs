//! Append-only indexed storage whose growth never copies existing elements.
use std::ops::{Index, IndexMut};

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

    pub fn push(&mut self, value: T) {
        if self.len / CHUNK_LEN == self.chunks.len() {
            self.chunks.push(Vec::with_capacity(CHUNK_LEN));
        }
        self.chunks[self.len / CHUNK_LEN].push(value);
        self.len += 1;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.chunks.get(index / CHUNK_LEN)?.get(index % CHUNK_LEN)
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator {
        (0..self.len).map(|index| &self[index])
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

    pub fn clear(&mut self) {
        for chunk in &mut self.chunks {
            chunk.clear();
        }
        self.len = 0;
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
