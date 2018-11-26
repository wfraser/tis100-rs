extern crate tis100;

#[macro_use] extern crate log;
extern crate structopt;
extern crate stderrlog;

use structopt::StructOpt;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::process::exit;

#[derive(StructOpt, Debug)]
struct Args {
    #[structopt(short="d", long="debug")]
    debug: bool,

    #[structopt(short="v", long="verbose", parse(from_occurrences))]
    verbose: usize,

    #[structopt(short="p", long="puzzle")]
    puzzle_num: Option<String>,

    #[structopt(parse(from_os_str))]
    savefile_path: PathBuf,
}

fn main() {
    let mut args = Args::from_args();
    if args.debug {
        if args.verbose != 0 {
            eprintln!("warning: debug flag overrides verbose");
        }
        args.verbose = 4;
    }
    let puzzle_num = args.puzzle_num
        .take()
        .unwrap_or_else(||
            args.savefile_path.file_name().unwrap()
                .to_str().unwrap()
                .split('.')
                .next()
                .unwrap()
                .to_owned());

    stderrlog::new()
        .verbosity(args.verbose)
        .init()
        .unwrap();

    let mut input = vec![];
    File::open(&args.savefile_path)
        .unwrap_or_else(|e| {
            error!("Error opening {:?}: {}", args.savefile_path, e);
            exit(1);
        })
        .read_to_end(&mut input)
        .unwrap_or_else(|e| {
            error!("Read error on {:?}: {}", args.savefile_path, e);
            exit(1);
        });

    let r = <rand::prng::ChaChaRng as rand::SeedableRng>::from_seed([0;32]);
    let p = tis100::puzzles::get_puzzle(&puzzle_num, 39, r).unwrap();

    let mut grid = tis100::grid::ComputeGrid::from_puzzle(p);

    let mut num_nodes = 0;
    let mut num_instructions = 0;
    let mut offset = 0;
    match tis100::assembly::parse_save_file(nom::types::CompleteByteSlice(&input)) {
        Ok((remaining, nodes)) => {
            if !remaining.is_empty() {
                error!("{} bytes unparsed at the end of input", remaining.len());
                warn!("parsed input comes out to: {:?}", nodes);
                exit(1);
            }

            for (id, asm) in nodes {
                info!("Save file node {}:", id.0);

                if !asm.is_empty() {
                    num_nodes += 1;
                    num_instructions += asm.iter()
                        .filter(|item|
                            if let tis100::instr::ProgramItem::Instruction(_) = item {
                                true
                            } else {
                                false
                            })
                        .count();
                }

                for i in &asm {
                    info!("\t{:?}", i);
                }

                let mut asm_iter = asm.into_iter();
                loop {
                    let programmed = grid.program_node(id.0 as usize + offset, &mut asm_iter);
                    if programmed {
                        info!("\tprogrammed node {}", id.0 as usize + offset);
                        break;
                    } else {
                        offset += 1;
                    }
                }
            }
        }
        Err(e) => {
            error!("parse error: {:?}", e);
        }
    }

    println!("{} nodes programmed", num_nodes);
    println!("{} total instructions", num_instructions);

    let mut cycle = 1;
    loop {
        if args.verbose > 1 {
            info!("--- start of cycle {} ---", cycle);
        } else if args.verbose == 1 {
            eprint!("\rcycle {}", cycle);
        }
        if let Some(correct) = grid.step() {
            if args.verbose == 1 {
                eprint!("\r");
            }
            if correct {
                print!("correct");
            } else {
                print!("incorrect");
            }
            println!(" solution in {} cycles", cycle);
            break;
        }
        //grid.print();
        cycle += 1;
    }
}
