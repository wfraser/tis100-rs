use crate::NodeId;
use crate::compute::ComputeNode;
use crate::instr::{Port, ProgramItem};
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

    pub fn step_all(&mut self) {
        println!("--- start of cycle ---");

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

        // Step all the compute nodes first. if they try to read this cycle, they'll wait until
        // next time to complete.
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
            let result = self.nodes[idx].step(avail_reads.as_mut_slice());
            println!("node {}: {:?}", idx, result);

            for (port, val) in &avail_reads {
                if val.is_none() {
                    // the value was taken
                    get_neighbor!(idx, *port).unwrap().pending_output = None;
                    // FIXME: check if the port was ANY and set LAST if it was from a compute node
                }
            }

            match result {
                NodeStepResult::WriteTo(port, val) => {
                    // FIXME: don't do this until end of the cycle; there needs to be propagation delay
                    self.nodes[idx].pending_output = Some((port, val));
                }
                _ => ()
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

            let result = node.step(avail_reads.as_mut_slice());
            println!("I/O port result: {:?}", result);

            if let NodeStepResult::WriteTo(port, val) = result {
                node.pending_output = Some((port, val));
            }

            for (_port, val) in &avail_reads { // FIXME: pointless loop; there can only be one
                if val.is_none() {
                    // the value was taken
                    self.nodes[id.0 as usize].pending_output = None;
                    // FIXME: check if the port was ANY and set LAST if it was from a compute node
                }
            }
        }
    }
}

#[derive(Debug)]
struct Node {
    inner: NodeType,
    pending_output: Option<(Port, i32)>, // port is relative to this node
}

#[derive(Debug)]
enum NodeType {
    Broken,
    Compute(ComputeNode),
    Input(InputNode),
    Output(OutputNode),
}

#[derive(Debug)]
pub enum NodeStepResult {
    ReadFrom(Port),
    WriteTo(Port, i32),
    Okay,
    Idle,
}

impl Node {
    pub fn new(inner: NodeType) -> Node {
        Node {
            inner,
            pending_output: None,
        }
    }

    pub fn step(&mut self, avail_reads: &mut [(Port, Option<i32>)]) -> NodeStepResult {
        if let Some((port, value)) = self.pending_output {
            NodeStepResult::WriteTo(port, value)
        } else {
            match &mut self.inner {
                NodeType::Broken => NodeStepResult::Idle,
                NodeType::Compute(comp) => comp.execute(avail_reads),
                NodeType::Input(inp) => inp.step(),
                NodeType::Output(out) => {
                    // FIXME: propagate the finished and failed state somehow.
                    match out.verify(avail_reads) {
                        VerifyState::Finished => panic!("done"),
                        VerifyState::Failed => panic!("failed"),
                        VerifyState::Okay => NodeStepResult::Okay,
                        VerifyState::Blocked => NodeStepResult::Idle,
                    }
                }
            }
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
}

// TODO: move to another file
#[derive(Debug)]
struct InputNode {
    values: Vec<i32>,
    pos: usize,
}

impl InputNode {
    pub fn new(values: Vec<i32>) -> Self {
        InputNode {
            values,
            pos: 0,
        }
    }

    pub fn step(&mut self) -> NodeStepResult {
        if self.pos < self.values.len() {
            self.pos += 1;
            NodeStepResult::WriteTo(Port::ANY, self.values[self.pos - 1])
        } else {
            NodeStepResult::Idle
        }
    }
}

#[derive(Debug)]
struct OutputNode {
    values: Vec<i32>,
    pos: usize,
}

impl OutputNode {
    pub fn new(values: Vec<i32>) -> Self {
        OutputNode {
            values,
            pos: 0,
        }
    }

    pub fn verify(&mut self, avail_reads: &mut [(Port, Option<i32>)]) -> VerifyState {
        if self.pos < self.values.len() {
            if let Some((_port, val)) = avail_reads.get_mut(0) {
                if val.take() == Some(self.values[self.pos]) {
                    self.pos += 1;
                    if self.pos == self.values.len() {
                        VerifyState::Finished
                    } else {
                        VerifyState::Okay
                    }
                } else {
                    VerifyState::Failed
                }
            } else {
                VerifyState::Blocked
            }
        } else {
            VerifyState::Finished
        }
    }
}

#[derive(Debug)]
enum VerifyState {
    Failed,
    Okay,
    Blocked,
    Finished,
}
