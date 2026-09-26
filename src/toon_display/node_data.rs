//! Dense metadata for values only, addressed by the original parsed node id.
//! One shared rank block per 64 parsed rows replaces metadata on closing rows.
use crate::flatjson::FlatJson;
use std::ops::{Index, IndexMut};
use std::sync::Arc;

struct Block {
    values: u64,
    before: usize,
}

struct NodeMap {
    blocks: Vec<Block>,
    len: usize,
}

impl NodeMap {
    fn slot(&self, node: usize) -> Option<usize> {
        let block = self.blocks.get(node / 64)?;
        let bit = 1u64 << (node % 64);
        (block.values & bit != 0)
            .then(|| block.before + (block.values & (bit - 1)).count_ones() as usize)
    }
}

pub struct NodeData<T> {
    map: Arc<NodeMap>,
    values: Vec<T>,
}

impl<T: Default + Clone> NodeData<T> {
    pub fn new(flat: &FlatJson) -> Self {
        let mut blocks = Vec::with_capacity(flat.0.len().div_ceil(64));
        let mut len = 0;
        for (node, row) in flat.0.iter().enumerate() {
            if node % 64 == 0 {
                blocks.push(Block {
                    values: 0,
                    before: len,
                });
            }
            if !row.is_closing_of_container() {
                blocks.last_mut().unwrap().values |= 1u64 << (node % 64);
                len += 1;
            }
        }
        Self {
            map: Arc::new(NodeMap { blocks, len }),
            values: vec![T::default(); len],
        }
    }
}

impl<T> NodeData<T> {
    pub fn filled_like<U: Default + Clone>(&self) -> NodeData<U> {
        NodeData {
            map: Arc::clone(&self.map),
            values: vec![U::default(); self.map.len],
        }
    }

    pub fn get(&self, node: usize) -> Option<&T> {
        self.map.slot(node).map(|slot| &self.values[slot])
    }
}

impl<T> Index<usize> for NodeData<T> {
    type Output = T;
    fn index(&self, node: usize) -> &T {
        &self.values[self.map.slot(node).expect("metadata requires a value node")]
    }
}

impl<T> IndexMut<usize> for NodeData<T> {
    fn index_mut(&mut self, node: usize) -> &mut T {
        let slot = self.map.slot(node).expect("metadata requires a value node");
        &mut self.values[slot]
    }
}
