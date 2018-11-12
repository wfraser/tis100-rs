use std::collections::BTreeMap;
use std::str::FromStr;

use crate::instr::*;

named!(
    node_id <&[u8], u8>,
    map_res!(
        take_while!(is_digit),
        |input| u8::from_str(unsafe { std::str::from_utf8_unchecked(input) }))
);

named!(
    pub node_tag <&[u8], NodeId>,
    do_parse!(
        tag!("@") >>
        id: node_id >>
        end_of_line >>
        (NodeId(id))
    )
);

named!(
    port <&[u8], Port>,
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
    register <&[u8], Register>,
    alt!(
        tag!("ACC") => { |_| Register::ACC }
        | tag!("NIL") => { |_| Register::NIL }
    )
);

fn is_digit(byte: u8) -> bool {
    byte >= b'0' && byte <= b'9'
}

named!(
    immediate <&[u8], i16>,
    map_res!(
        recognize!(tuple!(opt!(tag!("-")), take_while!(is_digit))),
        |input| {
            match i16::from_str(unsafe { std::str::from_utf8_unchecked(input) }) {
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
    label <&[u8], &str>,
    map!(take_while!(is_label_char),
        |bytes| unsafe { std::str::from_utf8_unchecked(bytes) }
    )
);

named!(
    source <&[u8], Src>,
    alt_complete!(
        register => { |r| Src::Register(r) }
        | port => { |p| Src::Port(p) }
        | immediate => { |n| Src::Immediate(n) }
    )
);

named!(
    dest <&[u8], Dst>,
    alt_complete!(
        register => { |r| Dst::Register(r) }
        | port => { |p| Dst::Port(p) }
    )
);

named!(
    comment <&[u8], &[u8]>,
    preceded!(tag!("#"), take_until!("\n"))
);

named!(
    end_of_line <&[u8], ()>,
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

named!(space <&[u8], &[u8]>, take_while!(nom::is_space));

named!(
    pub instruction <&[u8], Instruction>,
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
            (match op {
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
    )
);

named!(
    pub program_item <&[u8], ProgramItem>,
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
            end_of_line >> // instruction MUST have an end-of-line
            (ProgramItem::Instruction(i))
        )
        | do_parse!(
            tag!("!") >>
            opt!(space) >>
            (ProgramItem::Breakpoint)
        )
    )
);

named!(
    pub parse_save_file <&[u8], BTreeMap<NodeId, Vec<ProgramItem>>>,
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
