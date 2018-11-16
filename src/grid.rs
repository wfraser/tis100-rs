use crate::NodeId;
use crate::compute::ComputeNode;
use crate::instr::{Port, ProgramItem};
use crate::io::{InputNode, OutputNode, VerifyState};
use crate::puzzles::{PUZZLE_WIDTH, PUZZLE_HEIGHT, Puzzle};

use std::collections::BTreeMap;

#[derive(Debug)]
pub struct ComputeGrid {
    nodes: Vec<Node>,
    external: BTreeMap<(NodeId, Port), Node>,
    width: usize,
    height: usize,
}

impl ComputeGrid {
    pub fn from_puzzle(p: Puzzle) -> ComputeGrid {
        let mut nodes = Vec::with_capacity(PUZZLE_WIDTH * PUZZLE_HEIGHT);
        for idx in 0 .. PUZZLE_WIDTH * PUZZLE_HEIGHT {
            let node = if p.bad_nodes.contains(&NodeId(idx as u8)) {
                Node::new(NodeType::Broken)
            } else {
                Node::new(NodeType::Compute(ComputeNode::new()))
            };
            nodes.push(node);
        }

        let mut external = BTreeMap::new();
        for ((id, port), data) in p.inputs.into_iter() {
            external.insert((id, port), Node::new(NodeType::Input(InputNode::new(data))));
        }
        for ((id, port), data) in p.outputs.into_iter() {
            external.insert((id, port), Node::new(NodeType::Output(OutputNode::new(data))));
        }

        ComputeGrid {
            nodes,
            external,
            width: PUZZLE_WIDTH,
            height: PUZZLE_HEIGHT,
        }
    }

    pub fn program_node(&mut self, idx: usize, program_items: impl Iterator<Item=ProgramItem>)
        -> bool
    {
        self.nodes[idx].program_node(program_items)
    }

    pub fn step(&mut self) {
        self.read();
        self.compute();
        self.write();
        self.advance();

        let mut all_verified = true;
        for node in self.external.values() {
            match node.verify_state() {
                Some(VerifyState::Finished) => (),
                Some(VerifyState::Failed) => {
                    panic!("incorrect output");
                }
                Some(VerifyState::Blocked) | Some(VerifyState::Okay) => { all_verified = false; }
                None => ()
            }
        }

        if all_verified {
            panic!("done!");
        }
    }

    pub fn read(&mut self) {
        println!("- read step -");

        macro_rules! get_neighbor {
            ($idx:expr, $port:expr) => {
                if let Some(node) = self.external.get_mut(&(NodeId($idx as u8), $port)) {
                    Some(node)
                } else {
                    match $port {
                        Port::UP => {
                            if $idx >= self.width {
                                Some(&mut self.nodes[$idx - self.width])
                            } else {
                                None
                            }
                        }
                        Port::LEFT => {
                            if $idx > 0 {
                                Some(&mut self.nodes[$idx - 1])
                            } else {
                                None
                            }
                        }
                        Port::RIGHT => {
                            if $idx < self.nodes.len() - 1 {
                                Some(&mut self.nodes[$idx + 1])
                            } else {
                                None
                            }
                        }
                        Port::DOWN => {
                            if $idx + self.width < self.nodes.len() {
                                Some(&mut self.nodes[$idx + self.width])
                            } else {
                                None
                            }
                        }
                        _ => panic!("can't get neighbor {:?} ya dingus", $port)
                    }
                }
            }
        }

        for idx in 0 .. self.nodes.len() {

            // get readable values from neighbors

            let mut avail_reads = vec![];

            macro_rules! add_value_from {
                ($attached_port:expr) => {
                    if let Some(node) = get_neighbor!(idx, $attached_port) {
                        if let Some((port, val)) = node.pending_output {
                            if port == $attached_port.opposite() || port == Port::ANY {
                                avail_reads.push(($attached_port, Some(val)));
                            }
                        }
                    }
                }
            }

            add_value_from!(Port::UP);
            add_value_from!(Port::LEFT);
            add_value_from!(Port::RIGHT);
            add_value_from!(Port::DOWN);

            // Step the node!

            let result = self.nodes[idx].read(avail_reads.as_mut_slice());
            println!("node {}: {:?}", idx, result);

            for (port, val) in &avail_reads {
                if val.is_none() {
                    // the value was taken
                    // FIXME: this usage of port is dubious: what if it is ANY?
                    get_neighbor!(idx, *port).unwrap().complete_write(*port);
                }
            }
        }

        // Now step the I/O nodes
        for ((id, rel_port), ref mut node) in &mut self.external {
            let mut avail_reads = vec![];

            if let Some((dest_port, value)) = self.nodes[id.0 as usize].pending_output {
                if dest_port == Port::ANY || dest_port == *rel_port {
                    avail_reads.push((rel_port.opposite(), Some(value))); // port doesn't matter actually
                }
            }

            let result = node.read(avail_reads.as_mut_slice());
            let iotype = match node.inner {
                NodeType::Input(_) => "input",
                NodeType::Output(_) => "output",
                _ => "<unknown>",
            };
            println!("{} port result: {:?}", iotype, result);

            for (_port, val) in &avail_reads { // FIXME: pointless loop; there can only be one
                if val.is_none() {
                    // the value was taken
                    self.nodes[id.0 as usize].complete_write(*rel_port);
                }
            }
        }
    }

    fn compute(&mut self) {
        println!("- compute step -");
        for idx in 0 .. self.nodes.len() {
            let result = self.nodes[idx].compute();
            println!("node {}: {:?}", idx, result);
        }
        for node in self.external.values_mut() {
            node.compute();
        }
    }

    fn write(&mut self) {
        println!("- write step -");

        for idx in 0 .. self.nodes.len() {
            let result = self.nodes[idx].write();
            println!("node {}: {:?}", idx, result);
            if let CycleStepResult::IO((port, value)) = result {
                self.nodes[idx].pending_output = Some((port, value));
            }
        }

        for node in self.external.values_mut() {
            let result = node.write();
            let iotype = match node.inner {
                NodeType::Input(_) => "input",
                NodeType::Output(_) => "output",
                _ => "<unknown>",
            };
            println!("{} port result: {:?}", iotype, result);

            if let CycleStepResult::IO((port, value)) = result {
                node.pending_output = Some((port, value));
            }
        }
    }

    fn advance(&mut self) {
        println!("- advance step -");
        for idx in 0 .. self.nodes.len() {
            let result = self.nodes[idx].advance();
            println!("node {}: {:?}", idx, result);
        }
        for node in self.external.values_mut() {
            node.advance();
        }
    }
}

#[derive(Debug)]
struct Node {
    inner: NodeType,
    step: CycleStep,
    pending_output: Option<(Port, i32)>, // port is relative to this node
}

#[derive(Debug)]
enum NodeType {
    Broken,
    Compute(ComputeNode),
    Input(InputNode),
    Output(OutputNode),
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum CycleStep {
    Read, Compute, Write, Advance
}

#[derive(Debug)]
pub enum CycleStepResult<Output> {
    NotProgrammed,
    IO(Output),
    Done,
    Blocked(CycleStep),
}

pub type ReadResult = CycleStepResult<Port>;
pub type ComputeResult = CycleStepResult<!>;
pub type WriteResult = CycleStepResult<(Port, i32)>;
pub type AdvanceResult = CycleStepResult<!>;

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

    pub fn complete_write(&mut self, port: Port) {
        assert_eq!(CycleStep::Write, self.step);
        self.step = CycleStep::Advance;
        self.pending_output = None;

        if let NodeType::Compute(node) = &mut self.inner {
            node.complete_write(port);
        }
    }
}

macro_rules! check_step {
    ($self:expr, $exp:expr) => {
        if $self.step != $exp {
            return CycleStepResult::Blocked($self.step);
        }
    }
}

macro_rules! advance_step {
    ($self:expr, $res:expr, $next:expr) => {
        match $res {
            CycleStepResult::Done | CycleStepResult::NotProgrammed => {
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
            NodeType::Broken => CycleStepResult::NotProgrammed,
            NodeType::Compute(n) => n.read(avail_reads),
            NodeType::Input(_) => CycleStepResult::Done,
            NodeType::Output(n) => {
                n.read(avail_reads)
                    .map(CycleStepResult::IO)
                    .unwrap_or(CycleStepResult::Done)
            }
        };
        advance_step!(self, res, CycleStep::Compute);
        res
    }

    fn compute(&mut self) -> ComputeResult {
        check_step!(self, CycleStep::Compute);
        let res = match &mut self.inner {
            NodeType::Broken => CycleStepResult::NotProgrammed,
            NodeType::Compute(n) => n.compute(),
            NodeType::Input(_) | NodeType::Output(_) => CycleStepResult::Done,
        };
        advance_step!(self, res, CycleStep::Write);
        res
    }

    fn write(&mut self) -> WriteResult {
        check_step!(self, CycleStep::Write);
        if let Some((port, value)) = self.pending_output {
            return CycleStepResult::IO((port, value));
        }

        let res = match &mut self.inner {
            NodeType::Broken => CycleStepResult::NotProgrammed,
            NodeType::Compute(n) => n.write(),
            NodeType::Input(n) => {
                n.write()
                    .map(CycleStepResult::IO)
                    .unwrap_or(CycleStepResult::Done)
            }
            NodeType::Output(_) => CycleStepResult::Done,
        };
        advance_step!(self, res, CycleStep::Advance);
        res
    }

    fn advance(&mut self) -> AdvanceResult {
        check_step!(self, CycleStep::Advance);
        let res = match &mut self.inner {
            NodeType::Broken => CycleStepResult::NotProgrammed,
            NodeType::Compute(n) => n.advance(),
            NodeType::Input(_) | NodeType::Output(_) => CycleStepResult::Done,
        };
        advance_step!(self, res, CycleStep::Read);
        res
    }
}
