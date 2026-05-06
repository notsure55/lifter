use anyhow::Result;
use lifter::*;
use simple_logger::SimpleLogger;

fn main() -> Result<()> {
    SimpleLogger::new().with_colors(true).init().unwrap();

    let dissassembler = Dissassembler::new();
    let mut function = dissassembler.dissassemble_function(&NO_LOOP_DATA, 0x1400014e0);
    function.fix_relocations(0x140001470)?;

    println!("{function}");

    Ok(())
}
