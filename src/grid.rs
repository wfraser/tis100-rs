use crate::NodeId;
use crate::compute::ComputeNode;
use crate::instr::{Port, ProgramItem};
use crate::io::{InputNode, OutputNode, VerifyState};
use crate::node::{Node, NodeType, NodeOps};
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
                        if let Some((port, val)) = node.pending_output() {
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

            if let Some((dest_port, value)) = self.nodes[id.0 as usize].pending_output() {
                if dest_port == Port::ANY || dest_port == *rel_port {
                    avail_reads.push((rel_port.opposite(), Some(value))); // port doesn't matter actually
                }
            }

            let result = node.read(avail_reads.as_mut_slice());
            println!("{} port result: {:?}", node.type_name(), result);

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
        }

        for node in self.external.values_mut() {
            let result = node.write();
            println!("{} port result: {:?}", node.type_name(), result);
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
