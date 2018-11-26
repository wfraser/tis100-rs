use std::collections::BTreeMap;
use std::str::FromStr;

use crate::instr::*;

type Input<'a> = nom::types::CompleteByteSlice<'a>;

named!(
    node_id <Input, u8>,
    map_res!(
        take_while!(is_digit),
        |input: Input| u8::from_str(unsafe { std::str::from_utf8_unchecked(input.0) }))
);

named!(
    pub node_tag <Input, NodeId>,
    do_parse!(
        tag!("@") >>
        id: node_id >>
        end_of_line >>
        (NodeId(id))
    )
);

named!(
    port <Input, Port>,
    alt_complete!(
        tag!("UP") => { |_| Port::UP }
        | tag!("DOWN") => { |_| Port::DOWN }
        | tag!("LEFT") => { |_| Port::LEFT }
        | tag!("RIGHT") => { |_| Port::RIGHT }
        | tag!("ANY") => { |_| Port::ANY }
        | tag!("LAST") => { |_| Port::LAST }
    )
);

named!(
    register <Input, Register>,
    alt!(
        tag!("ACC") => { |_| Register::ACC }
        | tag!("NIL") => { |_| Register::NIL }
    )
);

fn is_digit(byte: u8) -> bool {
    byte >= b'0' && byte <= b'9'
}

named!(
    immediate <Input, i16>,
    map_res!(
        recognize!(tuple!(opt!(tag!("-")), take_while!(is_digit))),
        |input: Input| {
            match i16::from_str(unsafe { std::str::from_utf8_unchecked(input.0) }) {
                Ok(n) if n < -999 || n > 999 => Err("number out of range"),
                Err(_) => Err("number out of range"),
                Ok(n) => Ok(n),
            }
        }
    )
);

fn is_label_char(byte: u8) -> bool {
    match byte {
        b'A'...b'Z' => true,
        b'0'...b'9' => true,
        _ => false,
    }
}

named!(
    label <Input, &str>,
    map!(take_while!(is_label_char),
        |bytes| unsafe { std::str::from_utf8_unchecked(bytes.0) }
    )
);

named!(
    source <Input, Src>,
    alt_complete!(
        register => { |r| Src::Register(r) }
        | port => { |p| Src::Port(p) }
        | immediate => { |n| Src::Immediate(n) }
    )
);

named!(
    dest <Input, Dst>,
    alt_complete!(
        register => { |r| Dst::Register(r) }
        | port => { |p| Dst::Port(p) }
    )
);

named!(
    comment <Input, Input>,
    preceded!(tag!("#"), take_until!("\n"))
);

named!(
    end_of_line <Input, ()>,
    do_parse!(
        space >>
        opt!(comment) >>
        pair!(opt!(tag!("\r")), tag!("\n")) >>
        many0!(
            complete!(
                tuple!(
                    space,
                    opt!(comment),
                    pair!(opt!(tag!("\r")), tag!("\n"))
                )
            )
        ) >>
        (())
    )
);

named!(space <Input, Input>, take_while!(nom::is_space));

named!(
    pub instruction <Input, Instruction>,
    alt_complete!(
        tag!("NOP") => { |_| Instruction::NOP }
        | do_parse!(
            tag!("MOV") >>
            space >>
            src: source >>
            opt!(space) >>
            tag!(",") >>
            opt!(space) >>
            dst: dest >>
            (Instruction::MOV(src, dst))
        )
        | tag!("SWP") => { |_| Instruction::SWP }
        | tag!("SAV") => { |_| Instruction::SAV }
        | do_parse!(
            tag!("ADD") >>
            space >>
            src: source >>
            (Instruction::ADD(src))
        )
        | do_parse!(
            tag!("SUB") >>
            space >>
            src: source >>
            (Instruction::SUB(src))
        )
        | tag!("NEG") => { |_| Instruction::NEG }
        | do_parse!(
            op: alt!(tag!("JMP") | tag!("JEZ") | tag!("JNZ") | tag!("JGZ") | tag!("JLZ")) >>
            space >>
            label: label >>
            (match op.0 {
                b"JMP" => Instruction::JMP,
                b"JEZ" => Instruction::JEZ,
                b"JNZ" => Instruction::JNZ,
                b"JGZ" => Instruction::JGZ,
                b"JLZ" => Instruction::JLZ,
                _ => unreachable!()
            }(label.to_owned()))
        )
        | do_parse!(
            tag!("JRO") >>
            space >>
            src: source >>
            (Instruction::JRO(src))
        )
        | tag!("HCF") => { |_| Instruction::HCF }
    )
);

named!(
    pub program_item <Input, ProgramItem>,
    alt_complete!(
        do_parse!(
            opt!(eat_separator!(b" \t\r\n")) >>
            label: label >>
            tag!(":") >>
            opt!(end_of_line) >>
            (ProgramItem::Label(label.to_owned()))
        )
        | do_parse!(
            opt!(eat_separator!(b" \t\r\n")) >>
            i: instruction >>
            dbg_dmp!(
                alt!( complete!(end_of_line) => {|_|()}
                    | eof!() => {|_|()})
                ) >> // instruction MUST have an end-of-line or EOF
            (ProgramItem::Instruction(i))
        )
        | do_parse!(
            tag!("!") >>
            opt!(space) >>
            (ProgramItem::Breakpoint)
        )
    )
);

/// Convenience function for tests: read instructions from a string slice.
pub fn program_items(input: &[u8]) -> Result<Vec<ProgramItem>, (&[u8], Vec<ProgramItem>)> {
    named!(parse <Input, Vec<ProgramItem>>, many0!(program_item));

    match parse(input.into()) {
        Ok((remaining, items)) => {
            if remaining.is_empty() {
                Ok(items)
            } else {
                Err((&remaining, items))
            }
        }
        Err(_) => Err((input, vec![])),
    }
}

named!(
    pub parse_save_file <Input, BTreeMap<NodeId, Vec<ProgramItem>>>,
    fold_many1!(
        complete!(
            pair!(
                node_tag,
                many0!(program_item)
            )
        ),
        BTreeMap::<NodeId, Vec<ProgramItem>>::new(),
        |mut acc: BTreeMap<_,_>, item: (NodeId, Vec<_>)| { acc.insert(item.0, item.1); acc }
    )
);
