use super::get_ins_len;
use iced_x86::{
    self, Code, Decoder, DecoderOptions, Encoder, Instruction, MemoryOperand, Mnemonic, OpKind,
    Register,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::vec::Vec;

use anyhow::Result;

#[derive(Debug)]
pub struct BasicBlocks {
    pub blocks: BTreeMap<u64, BasicBlock>,
    //             Old address to new address
    address_map: BTreeMap<u64, u64>,
}

impl BasicBlocks {
    pub fn new() -> Self {
        Self {
            blocks: BTreeMap::new(),
            address_map: BTreeMap::new(),
        }
    }
    pub fn contains_key(&self, address: u64) -> bool {
        for (_, block) in self.blocks.iter() {
            if block.instructions.contains_key(&address) {
                return true;
            }
        }

        false
    }
    pub fn remove(&mut self, address: u64) -> Option<u64> {
        for (addr, block) in self.blocks.iter_mut() {
            if let Some(ins) = block.instructions.remove(&address) {
                log::info!("Removed {:X} {ins}", address);
                return Some(*addr);
            }
        }

        None
    }
    pub fn insert(&mut self, key: u64, block: BasicBlock) {
        self.blocks.insert(key, block);
    }
    pub fn find_preds(&self, block_addr: u64) -> BTreeSet<u64> {
        let mut preds = BTreeSet::new();

        for (addr, block) in self.blocks.iter() {
            if block.successors.contains(&block_addr) {
                log::info!("Found predecessor for {block_addr:X} {addr:X}");
                preds.insert(*addr);
            }
        }

        preds
    }
    pub fn insert_successors(&mut self, new_successors: &[u64], address_block: u64) {
        if let Some(block) = self.blocks.get_mut(&address_block) {
            block.successors.clear();
            block.successors.extend(new_successors);
        }
    }

    pub fn fix_rip_relatives(&mut self, new_addr: u64) -> Result<()> {
        let mut offset = 0;

        for (block_addr, block) in self.blocks.iter_mut() {
            let mut new_instructions: BTreeMap<u64, Instruction> = BTreeMap::new();

            for (_, ins) in block.instructions.iter() {
                if ins.is_jmp_near()
                    || ins.is_jcc_near()
                    || ins.is_jmp_near()
                    || ins.is_jmp_far()
                    || ins.is_jmp_near_indirect()
                    || ins.is_jmp_far_indirect()
                {
                    log::info!("Found jmp patching!");

                    let instructions = create_jmp_from_indirect_jmp(ins)?;

                    new_instructions.extend(&instructions);
                } else if ins.is_call_near()
                    || ins.is_call_far()
                    || ins.is_call_near_indirect()
                    || ins.is_call_far_indirect()
                {
                    if ins.op0_kind() == OpKind::Register {
                        log::info!("Found call to register! {ins}");
                    } else {
                        log::info!("Found call patching!");
                        let instructions = create_call_from_indirect_call(ins)?;

                        new_instructions.extend(&instructions);
                    }
                } else if ins.is_ip_rel_memory_operand() {
                    log::info!("Found relative instruction patching!");
                    new_instructions.extend(&create_mov_64_from_relative_lea(ins)?);
                }
            }

            block.instructions.append(&mut new_instructions);

            let mut encoder = Encoder::new(64);
            for (addr, ins) in block.instructions.iter() {
                self.address_map.insert(*addr, new_addr + offset);
                offset += encoder.encode(&ins, new_addr + offset)? as u64;
            }

            let bytes = encoder.take_buffer();
            let mut decoder =
                Decoder::with_ip(64, &bytes, new_addr + offset - bytes.len() as u64, 0);

            block.instructions.clear();

            for ins in decoder.iter() {
                block.instructions.insert(ins.ip(), ins);
            }
        }

        Ok(())
    }
    pub fn fix_short_jmps(&mut self) -> Result<()> {
        for (block_addr, block) in self.blocks.iter_mut() {
            for (addr, ins) in block.instructions.iter_mut() {
                if ins.is_jmp_short_or_near() || ins.is_jcc_short_or_near() {
                    let displacement = ins.memory_displacement64();

                    if let Some(new_displacement) = self.address_map.get(&displacement) {
                        log::info!("Setting new displacement {new_displacement:X} for {ins}");
                        ins.set_memory_displacement64(*new_displacement);
                        log::info!("New short jmp = {ins}");
                    }
                }
            }
        }

        Ok(())
    }
}

pub fn create_mov_64_from_relative_lea(ins: &Instruction) -> Result<BTreeMap<u64, Instruction>> {
    let mut new_instructions: BTreeMap<u64, Instruction> = BTreeMap::new();

    let op0 = ins.op0_register();

    log::info!("Old Instruction = {ins}");

    let mut new_ins = Instruction::with2(Code::Mov_r64_imm64, op0, ins.memory_displacement64())?;

    let len = get_ins_len(&new_ins)?;
    new_ins.set_len(len);
    new_ins.set_ip(ins.ip());

    new_instructions.insert(new_ins.ip(), new_ins);

    log::info!("New Instruction = {new_ins}");

    Ok(new_instructions)
}

pub fn create_call_from_indirect_call(ins: &Instruction) -> Result<BTreeMap<u64, Instruction>> {
    let mut new_instructions: BTreeMap<u64, Instruction> = BTreeMap::new();

    log::info!("Old Instruction = {ins}");

    let mut new_ins1 = Instruction::with2(
        Code::Mov_r64_imm64,
        Register::RAX,
        ins.memory_displacement64(),
    )?;

    let len = get_ins_len(&new_ins1)?;
    new_ins1.set_len(len);
    new_ins1.set_ip(ins.ip());

    let mut new_ins2 = Instruction::with1(Code::Call_rm64, Register::RAX)?;

    let len = get_ins_len(&new_ins2)?;
    new_ins2.set_len(len);
    new_ins2.set_ip(ins.ip() + new_ins1.len() as u64);

    new_instructions.insert(new_ins1.ip(), new_ins1);
    new_instructions.insert(new_ins2.ip(), new_ins2);

    log::info!("New Instructions = {new_ins1} {new_ins2}");

    Ok(new_instructions)
}

pub fn create_jmp_from_indirect_jmp(ins: &Instruction) -> Result<BTreeMap<u64, Instruction>> {
    let mut new_instructions: BTreeMap<u64, Instruction> = BTreeMap::new();

    log::info!("Old Instruction = {ins}");

    let mut new_ins1 = Instruction::with2(
        Code::Mov_r64_imm64,
        Register::RAX,
        ins.memory_displacement64(),
    )?;

    let len = get_ins_len(&new_ins1)?;
    new_ins1.set_len(len);
    new_ins1.set_ip(ins.ip());

    let mut new_ins2 = Instruction::with1(Code::Jmp_rm64, Register::RAX)?;

    let len = get_ins_len(&new_ins2)?;
    new_ins2.set_len(len);
    new_ins2.set_ip(ins.ip() + new_ins1.len() as u64);

    new_instructions.insert(new_ins1.ip(), new_ins1);
    new_instructions.insert(new_ins2.ip(), new_ins2);

    log::info!("New Instructions = {new_ins1} {new_ins2}");

    Ok(new_instructions)
}

#[derive(Debug)]
pub struct BasicBlock {
    pub instructions: BTreeMap<u64, Instruction>,
    pub successors: BTreeSet<u64>,
}

impl fmt::Display for BasicBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Address: {:X}\n", self.start_addr());
        for (address, ins) in self.instructions.iter() {
            write!(f, "  {:X} {}\n", address, ins);
        }

        Ok(())
    }
}

impl BasicBlock {
    pub fn new(
        ip: u64,
        bytes: &[u8],
        work_list: &Vec<u64>,
        basic_blocks: &mut BasicBlocks,
    ) -> Option<(Self, BTreeSet<u64>)> {
        let mut decoder = Decoder::with_ip(64, bytes, ip, 0);

        let mut instructions = BTreeMap::new();
        let mut branches = BTreeSet::new();
        let mut successors = BTreeSet::new();

        for ins in decoder.iter() {
            instructions.insert(ins.ip(), ins);

            // When parsing blocks, if we take a branch that starts in the middle of another block take its instructions.
            if basic_blocks.contains_key(ins.ip()) {
                if let Some(address) = basic_blocks.remove(ins.ip()) {
                    basic_blocks.insert_successors(&[ip], address);
                }
            }

            // if at the return that signals end of a block also
            if ins.mnemonic() == Mnemonic::Ret {
                branches.insert(ins.next_ip());
                break;
            }

            // all blocks should end in a jmp short or jcc
            if ins.is_jcc_short_or_near() {
                branches.insert(ins.memory_displacement64());
                branches.insert(ins.next_ip());

                successors.insert(ins.memory_displacement64());
                successors.insert(ins.next_ip());

                break;
            }

            if ins.is_jmp_short() {
                branches.insert(ins.memory_displacement64());
                successors.insert(ins.memory_displacement64());
            }

            // checks to see if we have already gone parsed the block or are going to parse the block
            if work_list.contains(&ins.next_ip())
                || basic_blocks.blocks.contains_key(&ins.next_ip())
            {
                break;
            }
        }

        if instructions.is_empty() {
            None
        } else {
            Some((
                Self {
                    instructions,
                    successors,
                },
                branches,
            ))
        }
    }
    pub fn start_addr(&self) -> u64 {
        let (addr, _ins) = self
            .instructions
            .first_key_value()
            .expect(&format!("Failed to get start addr for {:X?}", self));
        *addr
    }
    pub fn end_addr(&self) -> u64 {
        let (addr, _ins) = self.instructions.last_key_value().unwrap();
        *addr
    }
}
