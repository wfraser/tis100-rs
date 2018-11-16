extern crate tis100;

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::exit;

fn puzzle_num_arg() -> Option<Result<i32, std::num::ParseIntError>> {
    std::env::args().nth(2).map(|s| s.parse::<i32>())
}

fn puzzle_num(path: &Path) -> i32 {
    puzzle_num_arg()
        .map(|res| res.unwrap_or_else(|e| {
            eprintln!("Invalid puzzle number: {}", e);
            exit(1);
        }))
        .unwrap_or_else(|| {
            path.file_name().unwrap()
            .to_str().unwrap()
            .split('.')
            .next().unwrap()
            .parse::<i32>()
            .unwrap_or_else(|_| {
                match puzzle_num_arg() {
                    Some(Ok(n)) => n,
                    Some(Err(e)) => {
                        eprintln!("Invalid puzzle number: {}", e);
                        exit(1);
                    }
                    None => {
                        eprintln!("Unable to figure out puzzle number from the file name.");
                        eprintln!("Please provide a puzzle number as an extra command line argument.");
                        exit(1);
                    }
                }
            })
        })
}

fn main() {
    let path = match std::env::args_os().skip(1).next() {
        Some(s) => PathBuf::from(s),
        None => {
            eprintln!("Usage: {} <save file> [<puzzle number>]", std::env::args().nth(0).unwrap());
            eprintln!(r"Look for saves under C:\Users\<you>\Documents\My Games\TIS-100\<random>\save");
            exit(1);
        }
    };

    let puzzle_num = puzzle_num(&path);
    println!("puzzle number: {:?}", puzzle_num);

    let mut input = vec![];
    File::open(&path)
        .unwrap_or_else(|e| {
            eprintln!("Error opening {:?}: {}", path, e);
            exit(1);
        })
        .read_to_end(&mut input)
        .unwrap_or_else(|e| {
            eprintln!("Read error on {:?}: {}", path, e);
            exit(1);
        });

    let r = <rand::prng::ChaChaRng as rand::SeedableRng>::from_seed([0;32]);
    let p = tis100::puzzles::get_puzzle(puzzle_num, 39, r).unwrap();
    println!("{:?}", p);

    let mut grid = tis100::grid::ComputeGrid::from_puzzle(p);

    let mut offset = 0;
    match tis100::assembly::parse_save_file(nom::types::CompleteByteSlice(&input)) {
        Ok((remaining, nodes)) => {
            if !remaining.is_empty() {
                println!("{} bytes unparsed at the end of input", remaining.len());
                exit(1);
            }

            for (id, asm) in nodes {
                println!("Save file node {}:", id.0);
                for i in &asm {
                    println!("\t{:?}", i);
                }

                let mut asm_iter = asm.into_iter();
                loop {
                    let programmed = grid.program_node(id.0 as usize + offset, &mut asm_iter);
                    if programmed {
                        println!("programmed node {}", id.0 as usize + offset);
                        break;
                    } else {
                        offset += 1;
                    }
                }
            }
        }
        Err(e) => {
            println!("parse error: {:?}", e);
        }
    }
    println!("{:#?}", grid);

    let mut cycle = 0;
    loop {
        println!("cycle {}", cycle);
        grid.step_all();
        cycle += 1;
    }
}