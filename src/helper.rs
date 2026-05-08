use anyhow::Result;
use iced_x86::{Decoder, DecoderOptions, Mnemonic, OpKind, Register};
use std::ptr::NonNull;

pub const MAX_FUNCTION_SIZE: usize = 0x1000;

pub fn calculate_function_size(func: NonNull<u8>) -> Result<usize> {
    let data = std::ptr::slice_from_raw_parts::<u8>(func.as_ptr(), MAX_FUNCTION_SIZE);

    let mut decoder = Decoder::new(64, unsafe { &*data }, DecoderOptions::NONE);

    let mut stack_decrement = 0;

    for ins in decoder.iter() {
        // first sub is usually sub of rsp by some amount when setting up stack frame
        if ins.mnemonic() == Mnemonic::Sub && stack_decrement == 0
        // will always be a register if we are perform a sub
        && ins.op0_register() == Register::RSP
        {
            for (idx, op) in ins.op_kinds().enumerate() {
                match op {
                    OpKind::Immediate8
                    | OpKind::Immediate16
                    | OpKind::Immediate32
                    | OpKind::Immediate64
                    | OpKind::Immediate8to64 => {
                        stack_decrement = ins.try_immediate(idx as u32)?;
                        log::info!("Found functon prologue {ins} ");
                    }
                    _ => (),
                }
            }
        }

        if ins.mnemonic() == Mnemonic::Add
            && stack_decrement != 0
            && ins.op0_register() == Register::RSP
        {
            for (idx, op) in ins.op_kinds().enumerate() {
                match op {
                    OpKind::Immediate8
                    | OpKind::Immediate16
                    | OpKind::Immediate32
                    | OpKind::Immediate64
                    | OpKind::Immediate8to64 => {
                        let stack_increment = ins.try_immediate(idx as u32)?;
                        if stack_increment == stack_decrement {
                            log::info!("Found function epilogue: {ins}");
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    Ok(0)
}

pub fn calculate_function_size_from_bytes(_func: &[u8]) -> usize {
    0
}

pub fn calculate_size_rel_to_ins(data: &[u8], len: usize) -> Option<usize> {
    // Length plus max instruction length so we dont chop off any instructions when overwriting bytes
    let decoder = Decoder::new(64, data, DecoderOptions::NONE).into_iter();

    let mut current_size = 0;
    for ins in decoder {
        current_size += ins.len();
        if current_size >= len {
            break;
        }
    }

    // SAFETY if we break from the loop without having a size higher than len we
    // will return None because there is not enough space to overwrite bytes
    if current_size < len {
        None
    } else {
        Some(current_size)
    }
}
