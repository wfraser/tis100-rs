use crate::grid::NodeStepResult;
use crate::instr::*;

use std::collections::HashMap;

#[derive(Debug)]
pub struct ComputeNode {
    instructions: Vec<Instruction>,
    labels: HashMap<String, usize>,
    acc: i32,
    bak: i32,
    pc: usize,
    last: Port,
}

impl ComputeNode {
    pub fn new() -> Self {
        ComputeNode {
            instructions: vec![],
            labels: HashMap::new(),
            acc: 0,
            bak: 0,
            pc: 0,
            last: Port::LAST, // this value invokes nasal demons
        }
    }

    pub fn load_assembly(&mut self, items: impl Iterator<Item=ProgramItem>) {
        for item in items {
            match item {
                ProgramItem::Instruction(i) => {
                    self.instructions.push(i);
                }
                ProgramItem::Label(s) => {
                    self.labels.insert(s, self.instructions.len());
                }
                ProgramItem::Breakpoint => (), // TODO
            }
        }

        for instr in &self.instructions {
            match instr {
                Instruction::JMP(l) | Instruction::JEZ(l) | Instruction::JNZ(l)
                    | Instruction::JGZ(l) | Instruction::JLZ(l) =>
                {
                    if !self.labels.contains_key(l) {
                        panic!("instruction references undefined label {:?}", l);
                    }
                },
                _ => ()
            }
        }
    }

    pub fn execute(&mut self, avail_reads: &mut [(Port, Option<i32>)]) -> NodeStepResult {

        macro_rules! src_value {
            ($src:expr) => {
                match $src {
                    Src::Register(Register::ACC) => self.acc,
                    Src::Register(Register::NIL) => 0,
                    Src::Port(port) => {
                        fn read(
                            port: Port,
                            avail_reads: &mut [(Port, Option<i32>)],
                            last: &mut Port,
                        ) -> Option<i32> {
                            for (src_port, value) in avail_reads {
                                if port == Port::ANY || port == *src_port {
                                    if port == Port::ANY {
                                        *last = *src_port;
                                    }
                                    return value.take();
                                }
                            }
                            None
                        }

                        let actual_port = if *port == Port::LAST {
                            if self.last == Port::LAST {
                                panic!("attempted to read from unset LAST port!");
                            }
                            self.last
                        } else {
                            *port
                        };

                        match read(actual_port, avail_reads, &mut self.last) {
                            Some(value) => value,
                            None => {
                                return NodeStepResult::ReadFrom(*port);
                            }
                        }
                    }
                    Src::Immediate(i) => i32::from(*i),
                }
            }
        }

        if let Some(instr) = self.instructions.get(self.pc) {
            match instr {
                Instruction::NOP => (),
                Instruction::MOV(src, dst) => {
                    let val = src_value!(src);

                    match dst {
                        Dst::Register(Register::ACC) => { self.acc = val; }
                        Dst::Register(Register::NIL) => (),
                        Dst::Port(port) => {
                            let actual_port = if *port == Port::LAST {
                                if self.last == Port::LAST {
                                    panic!("attempted to write to unset LAST port");
                                }
                                self.last
                            } else {
                                *port
                            };

                            // TODO: if port is ANY, grid needs to set LAST to whoever atually read
                            // it.

                            self.pc += 1;
                            if self.pc >= self.instructions.len() {
                                self.pc = 0;
                            }
                            return NodeStepResult::WriteTo(actual_port, val);
                        }
                    }
                }
                Instruction::SWP => {
                    std::mem::swap(&mut self.acc, &mut self.bak);
                }
                Instruction::SAV => {
                    self.bak = self.acc;
                }
                Instruction::ADD(src) => {
                    let rhs = src_value!(src);
                    self.acc += rhs;
                }
                Instruction::SUB(src) => {
                    let rhs = src_value!(src);
                    self.acc -= rhs;
                }
                Instruction::NEG => {
                    self.acc = -self.acc;
                }
                Instruction::JMP(label) => { self.pc = self.labels[label]; }
                Instruction::JEZ(label) => if self.acc == 0 { self.pc = self.labels[label]; }
                Instruction::JNZ(label) => if self.acc != 0 { self.pc = self.labels[label]; }
                Instruction::JGZ(label) => if self.acc > 0 { self.pc = self.labels[label]; }
                Instruction::JLZ(label) => if self.acc < 0 { self.pc = self.labels[label]; }
                Instruction::JRO(src) => {
                    let off = src_value!(src);
                    if off < 0 {
                        self.pc -= -off as usize;
                    } else {
                        self.pc += off as usize;
                    }
                }
            }

            self.pc += 1;
            if self.pc >= self.instructions.len() {
                self.pc = 0;
            }

            NodeStepResult::Okay
        } else {
            NodeStepResult::Idle
        }
    }
}
