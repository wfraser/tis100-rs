extern crate tis100;

fn main() {
    let input = b"@0


@1


@2


@3


@4


@5
MOV ACC,DOWN
ADD 1

@6


@7


@8
0: MOV 30,ACC
LOOP: JEZ DONE
SUB 1
MOV 3,RIGHT
JMP LOOP

!DONE:MOV -1,RIGHT
JMP 0

@9
DONE: MOV 0,DOWN
MOV UP,DOWN

LOOP: MOV LEFT,ACC
MOV ACC,DOWN
JGZ LOOP

@10

";

    match tis100::assembly::parse_save_file(input) {
        Ok((remaining, nodes)) => {
            for (id, instrs) in nodes {
                println!("Node {}:", id.0);
                for i in instrs {
                    println!("\t{:?}", i);
                }
            }
            if !remaining.is_empty() {
                println!("{} bytes unparsed at the end", remaining.len());
            }
        }
        Err(e) => {
            println!("parse error: {:?}", e);
        }
    }
}
