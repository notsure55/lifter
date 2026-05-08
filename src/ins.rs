use super::get_ins_len;
use iced_x86::{self, Code, Instruction, Register};

use anyhow::Result;
use std::collections::BTreeMap;

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
    new_ins2.set_ip(ins.ip() + 1);

    new_instructions.insert(new_ins1.ip(), new_ins1);
    new_instructions.insert(new_ins2.ip(), new_ins2);

    log::info!("New Instructions = {new_ins1} {new_ins2}");

    Ok(new_instructions)
}

pub fn create_jmp_from_indirect_jmp(ins: &Instruction) -> Result<BTreeMap<u64, Instruction>> {
    let mut new_instructions: BTreeMap<u64, Instruction> = BTreeMap::new();

    log::info!("Old Instruction = {ins}");

    let indirect_value = unsafe { *(ins.memory_displacement64() as *const u64) };

    let mut new_ins1 = Instruction::with2(Code::Mov_r64_imm64, Register::RAX, indirect_value)?;

    let len = get_ins_len(&new_ins1)?;
    new_ins1.set_len(len);
    new_ins1.set_ip(ins.ip());

    let mut new_ins2 = Instruction::with1(Code::Jmp_rm64, Register::RAX)?;

    let len = get_ins_len(&new_ins2)?;
    new_ins2.set_len(len);
    new_ins2.set_ip(ins.ip() + 1);

    new_instructions.insert(new_ins1.ip(), new_ins1);
    new_instructions.insert(new_ins2.ip(), new_ins2);

    log::info!("New Instructions = {new_ins1} {new_ins2}");

    Ok(new_instructions)
}

pub fn create_jmp_from_relative_jmp(ins: &Instruction) -> Result<BTreeMap<u64, Instruction>> {
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
    new_ins2.set_ip(ins.ip() + 1);

    new_instructions.insert(new_ins1.ip(), new_ins1);
    new_instructions.insert(new_ins2.ip(), new_ins2);

    log::info!("New Instructions = {new_ins1} {new_ins2}");

    Ok(new_instructions)
}
