#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeId(pub u8);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Port {
    UP, DOWN, LEFT, RIGHT, ANY, LAST,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Register {
    ACC, NIL,
    // excludes BAK because it cannot be addressed
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Src {
    Register(Register),
    Port(Port),
    Immediate(i16),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Dst {
    Register(Register),
    Port(Port),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    NOP,
    MOV(Src, Dst),
    SWP,
    SAV,
    ADD(Src),
    SUB(Src),
    NEG,
    JMP(String),
    JEZ(String),
    JNZ(String),
    JGZ(String),
    JLZ(String),
    JRO(Src),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramItem {
    Instruction(Instruction),
    Label(String),
    Breakpoint,
}
