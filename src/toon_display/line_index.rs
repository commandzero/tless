//! Ranked line contributions, one bit per parsed node rather than one row per line.
//! Subtree counts are rank differences; select finds distant rows without walking siblings.
use std::ops::Range;

#[derive(Clone, Default)]
struct Block {
    bits: u64,
    before: usize,
}

pub struct LineIndex {
    blocks: Vec<Block>,
    total: usize,
}

impl LineIndex {
    pub fn new(nodes: usize) -> Self {
        Self {
            blocks: vec![Block::default(); nodes.div_ceil(64)],
            total: 0,
        }
    }

    pub fn reset(&mut self, source: &Self) {
        self.blocks.clone_from(&source.blocks);
        self.total = source.total;
    }

    pub fn insert(&mut self, node: usize) {
        self.blocks[node / 64].bits |= 1 << (node % 64);
    }

    pub fn clear(&mut self, range: Range<usize>) {
        if range.is_empty() {
            return;
        }
        let first = range.start / 64;
        let last = (range.end - 1) / 64;
        for index in first..=last {
            let low = if index == first { range.start % 64 } else { 0 };
            let high = if index == last {
                (range.end - 1) % 64 + 1
            } else {
                64
            };
            let mask = (u64::MAX << low) & (u64::MAX >> (64 - high));
            self.blocks[index].bits &= !mask;
        }
    }

    /// Finish mutations before any rank/select queries.
    pub fn finish(&mut self) {
        let mut total = 0;
        for block in &mut self.blocks {
            block.before = total;
            total += block.bits.count_ones() as usize;
        }
        self.total = total;
    }

    pub fn rank(&self, node: usize) -> usize {
        self.blocks.get(node / 64).map_or(self.total, |block| {
            block.before + (block.bits & ((1u64 << (node % 64)) - 1)).count_ones() as usize
        })
    }

    pub fn select(&self, line: usize) -> Option<usize> {
        if line >= self.total {
            return None;
        }
        let index = self
            .blocks
            .partition_point(|block| block.before + block.bits.count_ones() as usize <= line);
        let block = &self.blocks[index];
        let mut bits = block.bits;
        for _ in block.before..line {
            bits &= bits - 1;
        }
        Some(index * 64 + bits.trailing_zeros() as usize)
    }
}
