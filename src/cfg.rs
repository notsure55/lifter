use std::collections::{BTreeMap, BTreeSet};

use super::basicblock::BasicBlocks;
use std::fmt;

#[derive(Debug)]
struct CFBlock {
    successors: BTreeSet<u64>,
    predecessors: BTreeSet<u64>,
}

impl CFBlock {
    pub fn new(successors: BTreeSet<u64>, predecessors: BTreeSet<u64>) -> Self {
        Self {
            successors,
            predecessors,
        }
    }
}

impl fmt::Display for CFBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, " Successors \n",);
        for succ in self.successors.iter() {
            write!(f, "  {succ:X}\n",);
        }

        write!(f, " Predecessors \n",);
        for pred in self.predecessors.iter() {
            write!(f, "  {pred:X}\n",);
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Cfg(BTreeMap<u64, CFBlock>);

impl Cfg {
    pub fn new(blocks: &BasicBlocks) -> Self {
        let mut cfg: BTreeMap<u64, CFBlock> = BTreeMap::new();

        for (addr, block) in blocks.blocks.iter() {
            let preds = blocks.find_preds(*addr);

            let cf_block = CFBlock::new(block.successors.clone(), preds);

            cfg.insert(*addr, cf_block);
        }

        Self { 0: cfg }
    }
}

impl fmt::Display for Cfg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (addr, cf_block) in self.0.iter() {
            write!(f, "Address: {addr:X}\n{cf_block}");
        }
        Ok(())
    }
}
