#![feature(never_type)]

#[macro_use] extern crate maplit;
#[macro_use] extern crate nom;
extern crate rand;

pub mod assembly;
pub mod compute;
pub mod grid;
pub mod instr;
pub mod io;
pub mod node;
pub mod puzzles;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeId(pub u8);
