use nom::{HexDisplay,IResult,Offset};
use parser::{self,block,header,Block};

#[derive(Debug,Clone,PartialEq)]
pub enum State {
    Initial,
    Error,
    // file_size, file offset, list
    Blocks(usize, usize, List),
}

#[derive(Debug,Clone,PartialEq)]
pub enum List {
    Nil,
    // end of list, current list, parent list
    Node(usize, parser::List, Box<List>),
}

pub fn advance(state: State, input: &[u8]) -> (usize, State) {
    match state {
        State::Initial => parse_initial(input),
        State::Blocks(file_size, offset, level) => parse_blocks(input, file_size, offset, level),
        _              => panic!("unimplemented state"),



    }
}

pub fn parse_initial(input: &[u8]) -> (usize, State) {
    match header(input) {
        IResult::Error(_)        => (0, State::Error),
        IResult::Incomplete(_)   => (0, State::Initial),
        IResult::Done(i, header) => (input.offset(i), State::Blocks(header.file_size as usize, input.offset(i), List::Nil)),
    }
}

pub fn parse_blocks(input: &[u8], file_size: usize, offset: usize, level: List) -> (usize, State) {
    //FIXME: handle closing list
    let sl = match level {
        List::Nil                   => input,
        List::Node(remaining, _, _) => &input[..(remaining - offset)]
    };

    match block(sl, offset, file_size as u32) {
        IResult::Error(_)        => (0, State::Error),
        IResult::Incomplete(_)   => (0, State::Blocks(file_size, offset, level)),
        IResult::Done(i, blk) => {
            let remaining_offset = input.offset(i);
            match blk {
                Block::Unimplemented => panic!("unimplemented block:\n{}", &input[..input.offset(i)].to_hex(16)),
                Block::Default       => panic!("default block:\n{}", &input[..input.offset(i)].to_hex(16)),
                Block::Avih(h)       => {
                    println!("got main AVI header: {:?}", h);
                    (remaining_offset, State::Blocks(file_size, offset + remaining_offset, level))
                },
                Block::List(size, l) => {
                    match level {
                        List::Nil => (remaining_offset,
                            State::Blocks(
                                file_size,
                                offset + remaining_offset,
                                List::Node(offset + remaining_offset + size, l, Box::new(List::Nil)))),
                        List::Node(sz, _, _) => {
                            if sz < offset + remaining_offset + size {
                                // the new list would be larger than the parent one
                                (remaining_offset, State::Error)
                            } else {
                                (remaining_offset,
                                State::Blocks(
                                    file_size,
                                    offset + remaining_offset,
                                    List::Node(offset + remaining_offset + size, l, Box::new(level))))
                            }
                        }
                    }
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const drop   : &'static [u8] = include_bytes!("../assets/drop.avi");
    const verona : &'static [u8] = include_bytes!("../assets/verona60avi56k.avi");

    #[test]
    fn state_initial() {
        let mut opt_state = Some(State::Initial);
        let mut offset    = 0usize;

        loop {
            if offset > drop.len() {
                println!("file ended");
                break;
            }

            println!("\nwill parse:\n{}\n", &drop[offset..200].to_hex(16));
            let state = opt_state.take().expect("should not be none here");
            let (mv, state) = advance(state, &drop[offset..]);

            println!("state is: {:?} (advancing {})", state, mv);
            offset += mv;

            opt_state = Some(state);
        }
    }

}
