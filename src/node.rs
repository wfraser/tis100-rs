use crate::compute::ComputeNode;
use crate::instr::{Port, ProgramItem};
use crate::io::{InputNode, OutputNode, VerifyState};

#[derive(Debug)]
pub struct Node {
    pub inner: NodeType,
    pub step: CycleStep,
    pub pending_output: Option<(Port, i32)>, // port is relative to this node
}

#[derive(Debug)]
pub enum NodeType {
    Broken,
    Compute(ComputeNode),
    Input(InputNode),
    Output(OutputNode),
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum CycleStep {
    Read, Compute, Write, Advance,
}

impl std::fmt::Display for CycleStep {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.pad(match self {
            CycleStep::Read => "READ",
            CycleStep::Compute => "COMP",
            CycleStep::Write => "WRTE",
            CycleStep::Advance => "ADVN",
        })
    }
}

#[derive(Debug)]
pub enum StepResult<Output> {
    NotProgrammed,
    IO(Output),
    Done,
    Blocked(CycleStep),
}

pub type ReadResult = StepResult<Port>;
pub type ComputeResult = StepResult<!>;
pub type WriteResult = StepResult<(Port, i32)>;
pub type AdvanceResult = StepResult<!>;

pub trait NodeOps {
    fn read(&mut self, avail_reads: &mut [(Port, Option<i32>)]) -> ReadResult;
    fn compute(&mut self) -> ComputeResult;
    fn write(&mut self) -> WriteResult;
    fn advance(&mut self) -> AdvanceResult;
}

impl Node {
    pub fn new(inner: NodeType) -> Node {
        Node {
            inner,
            step: CycleStep::Read,
            pending_output: None,
        }
    }

    pub fn program_node(&mut self, program_items: impl Iterator<Item=ProgramItem>) -> bool {
        match &mut self.inner {
            NodeType::Broken => false,
            NodeType::Compute(comp) => {
                comp.load_assembly(program_items);
                true
            }
            _ => panic!("attempted to program an I/O node somehow"),
        }
    }

    pub fn verify_state(&self) -> Option<VerifyState> {
        if let NodeType::Output(out) = &self.inner {
            Some(out.verified())
        } else {
            None
        }
    }

    pub fn pending_output(&self) -> Option<(Port, i32)> {
        self.pending_output
    }

    pub fn complete_write(&mut self, port: Port) {
        assert_eq!(CycleStep::Write, self.step);
        self.step = CycleStep::Advance;
        self.pending_output = None;

        if let NodeType::Compute(node) = &mut self.inner {
            node.complete_write(port);
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self.inner {
            NodeType::Broken => "broken",
            NodeType::Compute(_) => "compute",
            NodeType::Input(_) => "input",
            NodeType::Output(_) => "output",
        }
    }
}
macro_rules! check_step {
    ($self:expr, $exp:expr) => {
        if $self.step != $exp {
            return StepResult::Blocked($self.step);
        }
    }
}

macro_rules! advance_step {
    ($self:expr, $res:expr, $next:expr) => {
        match $res {
            StepResult::Done | StepResult::NotProgrammed => {
                $self.step = $next
            }
            _ => ()
        }
    }
}

impl NodeOps for Node {
    fn read(&mut self, avail_reads: &mut [(Port, Option<i32>)]) -> ReadResult {
        check_step!(self, CycleStep::Read);
        let res = match &mut self.inner {
            NodeType::Broken => StepResult::NotProgrammed,
            NodeType::Compute(n) => n.read(avail_reads),
            NodeType::Input(_) => StepResult::Done,
            NodeType::Output(n) => {
                n.read(avail_reads)
                    .map(StepResult::IO)
                    .unwrap_or(StepResult::Done)
            }
        };
        advance_step!(self, res, CycleStep::Compute);
        res
    }

    fn compute(&mut self) -> ComputeResult {
        check_step!(self, CycleStep::Compute);
        let res = match &mut self.inner {
            NodeType::Broken => StepResult::NotProgrammed,
            NodeType::Compute(n) => n.compute(),
            NodeType::Input(_) | NodeType::Output(_) => StepResult::Done,
        };
        advance_step!(self, res, CycleStep::Write);
        res
    }

    fn write(&mut self) -> WriteResult {
        check_step!(self, CycleStep::Write);
        if let Some((port, value)) = self.pending_output {
            return StepResult::IO((port, value));
        }

        let res = match &mut self.inner {
            NodeType::Broken => StepResult::NotProgrammed,
            NodeType::Compute(n) => n.write(),
            NodeType::Input(n) => {
                n.write()
                    .map(StepResult::IO)
                    .unwrap_or(StepResult::Done)
            }
            NodeType::Output(_) => StepResult::Done,
        };

        if let StepResult::IO((port, value)) = res {
            self.pending_output = Some((port, value));
        }

        advance_step!(self, res, CycleStep::Advance);
        res
    }

    fn advance(&mut self) -> AdvanceResult {
        check_step!(self, CycleStep::Advance);
        let res = match &mut self.inner {
            NodeType::Broken => StepResult::NotProgrammed,
            NodeType::Compute(n) => n.advance(),
            NodeType::Input(_) | NodeType::Output(_) => StepResult::Done,
        };
        advance_step!(self, res, CycleStep::Read);
        res
    }
}
