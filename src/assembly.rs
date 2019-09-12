use crate::instr::*;
use nom::IResult;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until, take_while};
use nom::character::complete::{digit1, line_ending, space0, space1};
use nom::combinator::{all_consuming, complete, opt, map, map_res, recognize, value};
use nom::error::ParseError;
use nom::multi::{fold_many1, many0, many0_count, many1_count};
use nom::sequence::{preceded, tuple};
use std::collections::BTreeMap;

fn node_id(input: &[u8]) -> IResult<&[u8], u8> {
    map_res(
        digit1,
        |i: &[u8]| unsafe { std::str::from_utf8_unchecked(i) }.parse::<u8>(),
    )(input)
}

fn node_tag(input: &[u8]) -> IResult<&[u8], SaveFileNodeId> {
    let (input, _) = tag(b"@")(input)?;
    let (input, id) = node_id(input)?;
    let (input, _) = end_of_line(input)?;
    Ok((input, SaveFileNodeId(id)))
}

fn port(input: &[u8]) -> IResult<&[u8], Port> {
    alt(
        (
            value(Port::UP, tag(b"UP")),
            value(Port::DOWN, tag(b"DOWN")),
            value(Port::LEFT, tag(b"LEFT")),
            value(Port::RIGHT, tag(b"RIGHT")),
            value(Port::ANY, tag(b"ANY")),
            value(Port::LAST, tag(b"LAST")),
        )
    )(input)
}

fn register(input: &[u8]) -> IResult<&[u8], Register> {
    alt(
        (
            value(Register::ACC, tag(b"ACC")),
            value(Register::NIL, tag(b"NIL")),
        )
    )(input)
}

fn immediate(input: &[u8]) -> IResult<&[u8], i16> {
    map_res(
        recognize(tuple((opt(tag(b"-")), digit1))),
        |bytes: &[u8]| {
            let s = unsafe { std::str::from_utf8_unchecked(bytes) };
            match s.parse::<i16>() {
                Ok(n) if n < -999 || n > 999 => Err("number out of range"),
                Err(_) => Err("number out of range"),
                Ok(n) => Ok(n),
            }
        }
    )(input)
}

fn is_label_char(byte: u8) -> bool {
    match byte {
        b'A' ..= b'Z' => true,
        b'0' ..= b'9' => true,
        b'-' => true,
        _ => false,
    }
}

fn label(input: &[u8]) -> IResult<&[u8], &str> {
    map(
        take_while(is_label_char),
        |bytes| unsafe { std::str::from_utf8_unchecked(bytes) }
    )(input)
}

fn source(input: &[u8]) -> IResult<&[u8], Src> {
    alt(
        (
            map(register, Src::Register),
            map(port, Src::Port),
            map(immediate, Src::Immediate),
        ),
    )(input)
}

fn dest(input: &[u8]) -> IResult<&[u8], Dst> {
    alt(
        (
            map(register, Dst::Register),
            map(port, Dst::Port),
        )
    )(input)
}

fn comment(input: &[u8]) -> IResult<&[u8], ()> {
    map(
        preceded(
            tag(b"#"),
            take_until(&b"\n"[..]),
        ),
        |_| ()
    )(input)
}

fn end_of_line(input: &[u8]) -> IResult<&[u8], ()> {
    map(
        many1_count(
            complete(
                tuple(
                    (space0, opt(comment), line_ending)
                )
            )
        ),
        |_| (),
    )(input)
}

fn comments_and_whitespace(input: &[u8]) -> IResult<&[u8], ()> {
    map(
        many0_count(
            alt(
                (
                    comment,
                    map(nom::character::complete::multispace1, |_| ()),
                )
            ),
        ),
        |_| ()
    )(input)
}

fn arg_sep(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt(
        (
            recognize(tuple(
                (
                    space0,
                    tag(b","),
                    space0,
                )
            )),
            space1,
        )
    )(input)
}

fn instruction(input: &[u8]) -> IResult<&[u8], Instruction> {
    alt(
        (
            value(Instruction::NOP, tag(b"NOP")),
            |input| {
                let (input, _) = tag(b"MOV")(input)?;
                let (input, _) = space1(input)?;
                let (input, src) = source(input)?;
                let (input, _) = arg_sep(input)?;
                let (input, dst) = dest(input)?;
                Ok((input, Instruction::MOV(src, dst)))
            },
            value(Instruction::SWP, tag(b"SWP")),
            value(Instruction::SAV, tag(b"SAV")),
            map(tuple((tag(b"ADD"), space1, source)), |(_,_,src)| Instruction::ADD(src)),
            map(tuple((tag(b"SUB"), space1, source)), |(_,_,src)| Instruction::SUB(src)),
            value(Instruction::NEG, tag(b"NEG")),
            map(tuple(
                    (
                        alt((tag(b"JMP"), tag(b"JEZ"), tag(b"JNZ"), tag(b"JGZ"), tag(b"JLZ"))),
                        space1,
                        label,
                    )
                ),
                |(op,_,label)| {
                    let ctor = match op {
                        b"JMP" => Instruction::JMP,
                        b"JEZ" => Instruction::JEZ,
                        b"JNZ" => Instruction::JNZ,
                        b"JGZ" => Instruction::JGZ,
                        b"JLZ" => Instruction::JLZ,
                        _ => unreachable!()
                    };
                    ctor(label.to_owned())
                }
            ),
            map(tuple((tag(b"JRO"), space1, source)), |(_,_,src)| Instruction::JRO(src)),
            value(Instruction::HCF, tag(b"HCF")),
        )
    )(input)
}

fn eof(input: &[u8]) -> IResult<&[u8], ()> {
    use nom::error::ErrorKind;
    if input.is_empty() {
        Ok((input, ()))
    } else {
        Err(nom::Err::Error(ParseError::from_error_kind(input, ErrorKind::Eof)))
    }
}

fn program_item(input: &[u8]) -> IResult<&[u8], ProgramItem> {
    alt(
        (
            map(tuple((comments_and_whitespace, label, tag(b":"), opt(end_of_line))),
                |(_, label, _, _)| ProgramItem::Label(label.to_owned())),
            map(tuple((comments_and_whitespace, instruction, alt((end_of_line, eof)))),
                |(_, inst, _)| ProgramItem::Instruction(inst)),
            value(ProgramItem::Breakpoint, tuple((tag(b"!"), space0))),
        )
    )(input)
}

pub fn program_items(input: &[u8]) -> Result<Vec<ProgramItem>, (&[u8], Vec<ProgramItem>)> {
    all_consuming(many0(program_item))(input)
        .map(|(_, items)| items)
        .map_err(|_| (input, Default::default()))
}

pub fn parse_save_file(input: &[u8])
    -> Result<
        BTreeMap<SaveFileNodeId, Vec<ProgramItem>>,
        (&[u8], BTreeMap<SaveFileNodeId, Vec<ProgramItem>>)>
{
    let result = fold_many1(
        tuple((node_tag, many0(program_item))),
        BTreeMap::<SaveFileNodeId, Vec<ProgramItem>>::new(),
        |mut acc, (node_id, items)| {
            acc.insert(node_id, items);
            acc
        }
    )(input);

    match result {
        Ok((&[], map)) => Ok(map),
        Ok((rest, map)) => {
            error!("Parse error: not all input processed");
            Err((rest, map))
        }
        Err(nom::Err::Incomplete(_needed)) => unreachable!("incomplete parse should not be possible when using 'complete' parser"),
        Err(nom::Err::Error((rest, err_kind))) | Err(nom::Err::Failure((rest, err_kind))) => {
            error!("Parse error: {:?}", err_kind);
            Err((rest, Default::default()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_end_of_line() {
        assert!(end_of_line(b"\n").is_ok());
        assert!(end_of_line(b"").is_err());
    }

    #[test]
    fn test_eof() {
        assert_eq!((), eof(b"").unwrap().1);
        assert!(eof(b"foo").is_err());
    }

    #[test]
    fn test_program_item() {
        assert!(program_item(b"# lol\nMOV UP,DOWN\n").is_ok());
        assert!(program_item(b"# lol\nMOV UP,DOWN garbage").is_err());
    }

    #[test]
    fn test_mov() {
        assert_eq!(Instruction::MOV(Src::Immediate(-325), Dst::Port(Port::LAST)),
            instruction(b"MOV -325, LAST").unwrap().1);
    }

    #[test]
    fn test_add() {
        assert_eq!(Instruction::ADD(Src::Immediate(-325)),
            instruction(b"ADD -325").unwrap().1);
    }

    #[test]
    fn test_port() {
        assert_eq!(Port::LAST, port(b"LAST").unwrap().1);
        assert!(port(b"").is_err());
    }

    #[test]
    fn test_immediate() {
        assert_eq!(-325, immediate(b"-325").unwrap().1);
    }

    #[test]
    fn test_node_tag() {
        assert_eq!(SaveFileNodeId(42), node_tag(b"@42\r\n# hello\r\n").unwrap().1);
    }

    #[test]
    fn test_arg_sep() {
        macro_rules! check {
            ($e:expr) => {
                assert_eq!(Ok((&[][..], &$e[..])), arg_sep(&$e[..]));
            }
        }
        check!(b",");
        check!(b" ");
        check!(b"    ");
        check!(b"  ,");
        check!(b",   ");
        check!(b"  ,  ");
    }
}
