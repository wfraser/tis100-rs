extern crate tis100;

extern crate rand;

use tis100::instr::*;

fn rng() -> impl rand::Rng + Clone {
    <rand::prng::ChaChaRng as rand::SeedableRng>::from_seed([0;32])
}

fn asm(input: &str) -> Vec<ProgramItem> {
    tis100::assembly::program_items(input.as_bytes()).expect("asm parse error")
}

trait VecExt<T> {
    fn single(self) -> T;
}

impl<T> VecExt<T> for Vec<T> {
    fn single(mut self) -> T {
        assert_eq!(1, self.len());
        self.drain(..).next().unwrap()
    }
}

#[test]
fn parse_test() {
    assert_eq!(
        ProgramItem::Instruction(
            Instruction::MOV(
                Src::Port(
                    Port::ANY),
                Dst::Port(
                    Port::LAST))),
        asm("MOV ANY, LAST").single());

    assert_eq!(
        ProgramItem::Instruction(
            Instruction::MOV(
                Src::Immediate(999),
                Dst::Register(
                    Register::NIL))),
        asm("MOV 999, NIL").single());
}

#[test]
fn parse_whitespace_test() {
    assert_eq!(
        ProgramItem::Instruction(
            Instruction::JEZ("0".to_owned())),
        asm("\n\t  # foo\n# bar\nJEZ\t0\n# whatever\n\n\n").single());
}

#[test]
#[should_panic(expected = "asm parse error")]
fn out_of_range_immediate() {
    asm("MOV 1000, ANY");
}

#[test]
fn connectivity_test() {
    let puz = tis100::puzzles::get_puzzle("DBG01", 39, rng()).unwrap();
    let mut grid = tis100::grid::ComputeGrid::from_puzzle(puz);

    // Move the value around in a serpentine motion.
    // In
    // → → → ↓
    // ↓ ← ← ←
    // → → → ↓
    //       Out
    grid.program_node(0, asm("MOV ANY, RIGHT"));
    grid.program_node(1, asm("MOV ANY, RIGHT"));
    grid.program_node(2, asm("MOV ANY, RIGHT"));
    grid.program_node(3, asm("MOV ANY, DOWN"));
    grid.program_node(7, asm("MOV ANY, LEFT"));
    grid.program_node(6, asm("MOV ANY, LEFT"));
    grid.program_node(5, asm("MOV ANY, LEFT"));
    grid.program_node(4, asm("MOV ANY, DOWN"));
    grid.program_node(8, asm("MOV ANY, RIGHT"));
    grid.program_node(9, asm("MOV ANY, RIGHT"));
    grid.program_node(10, asm("MOV ANY, RIGHT"));
    grid.program_node(11, asm("MOV ANY, DOWN"));

    let mut cycle = 1;
    loop {
        match grid.step() {
            None => { cycle += 1; }
            Some(true) => { break; }
            Some(false) => { panic!("incorrect result"); }
        }
        if cycle > 20 {
            panic!("too many cycles");
        }
    }
    assert_eq!(20, cycle, "wrong number of cycles");
}
