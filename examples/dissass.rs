use anyhow::Result;
use lifter::*;
use simple_logger::SimpleLogger;

fn main() -> Result<()> {
    SimpleLogger::new().with_colors(true).init().unwrap();

    let dissassembler = Dissassembler::new();
    let mut function = dissassembler.dissassemble_function(&PRINT_DATA, 0x140001470);
    function.fix_relocations()?;

    println!("{bytes:X?}");

    Ok(())
}
