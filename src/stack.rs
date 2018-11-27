use crate::node::{StepResult, ReadResult, ComputeResult, WriteResult, AdvanceResult, NodeOps};
use crate::instr::Port;

#[derive(Debug, Default)]
pub struct StackNode {
    values: Vec<i32>,
}

impl NodeOps for StackNode {
    fn read(&mut self, avail_reads: &mut [(Port, Option<i32>)]) -> ReadResult {
        for (src_port, value) in avail_reads {
            let value = value.take().unwrap();
            debug!("stack node read {} from {}", value, src_port);
            self.values.push(value);
            return StepResult::Done
        }
        StepResult::Done // don't return IO because we don't want to get stuck here
    }

    fn compute(&mut self) -> ComputeResult {
        StepResult::NotProgrammed
    }

    fn write(&mut self) -> WriteResult {
        if let Some(value) = self.values.last() {
            StepResult::IO((Port::ANY, *value))
        } else {
            StepResult::Done
        }
    }

    fn advance(&mut self) -> AdvanceResult {
        self.values.pop();
        StepResult::Done
    }

}
