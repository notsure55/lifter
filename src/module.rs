use anyhow::Result;
use iced_x86::Encoder;
use std::fmt;

use super::basicblock::{BasicBlock, BasicBlocks};
use super::cfg::Cfg;

#[derive(Debug)]
pub struct Module {
    blocks: BasicBlocks,
    cfg: Cfg,
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (_, block) in self.blocks.blocks.iter() {
            write!(f, "{}\n", block)?;
        }

        write!(f, "{}", self.cfg)?;

        Ok(())
    }
}

impl Module {
    pub fn new(bytes: &[u8], ip: u64) -> Self {
        let mut blocks = BasicBlocks::new();
        let mut work_list = vec![ip];

        while let Some(addr) = work_list.pop() {
            if let Some((block, branches)) = BasicBlock::new(
                addr,
                &bytes[addr as usize - ip as usize..],
                &work_list,
                &mut blocks,
            ) {
                for branch in branches.iter() {
                    if !blocks.blocks.contains_key(branch) {
                        work_list.push(*branch);
                    }
                }

                blocks.insert(block.start_addr(), block);
            } else {
                log::warn!("Reached none basic block at: {addr:?}");
            }
        }

        let cfg = Cfg::new(&blocks);

        Self { blocks, cfg }
    }
    pub fn fix_relocations(&mut self) -> Result<&mut Self> {
        self.start_addr();

        self.blocks.fix_rip_relatives()?;
        self.blocks.fix_short_jmps()?;

        Ok(self)
    }
    pub fn start_addr(&self) -> u64 {
        let (addr, _) = self.blocks.blocks.first_key_value().unwrap();
        *addr
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut encoder = Encoder::new(64);

        for (_, block) in self.blocks.blocks.iter() {
            for (_, ins) in block.instructions.iter() {
                encoder.encode(ins, ins.ip())?;
            }
        }

        Ok(encoder.take_buffer())
    }
}
