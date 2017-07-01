use nom::{HexDisplay,IResult,Offset};
use parser::{self,block,header,Block};

#[derive(Debug,Clone,PartialEq)]
pub enum State {
    Initial,
    Error,
    Blocks(Context),
}

#[derive(Debug,Clone,PartialEq)]
pub enum List {
    Nil,
    // end of list, current list, parent list
    Node(usize, parser::List, Box<List>),
}

#[derive(Debug,Clone,PartialEq)]
pub struct Context {
    file_size:     usize,
    stream_offset: usize,
    level:         List,
}

pub fn advance(state: State, input: &[u8]) -> (usize, State) {
    match state {
        State::Initial => parse_initial(input),
        State::Blocks(context) => parse_blocks(input, context),
        _              => panic!("unimplemented state"),



    }
}

pub fn parse_initial(input: &[u8]) -> (usize, State) {
    match header(input) {
        IResult::Error(_)        => (0, State::Error),
        IResult::Incomplete(_)   => (0, State::Initial),
        IResult::Done(i, header) => (input.offset(i), State::Blocks(Context {
            file_size: header.file_size as usize,
            stream_offset: input.offset(i),
            level: List::Nil
        })),
    }
}

pub fn parse_blocks(input: &[u8], mut ctx: Context) -> (usize, State) {
    //FIXME: handle closing list
    let sl = match ctx.level {
        List::Nil                   => input,
      // min of offset and input length?
        List::Node(remaining, _, _) => &input[..(remaining - ctx.stream_offset)]
    };

    match block(sl, ctx.stream_offset, ctx.file_size as u32) {
        IResult::Error(e)        => {
          println!("got error: {:?}", e);
          (0, State::Error)
        },
        IResult::Incomplete(_)   => (0, State::Blocks(ctx)),
        IResult::Done(i, blk) => {
            let advancing = input.offset(i);
            ctx.stream_offset += advancing;
            match blk {
                Block::Unimplemented => panic!("unimplemented block:\n{}", &input[..advancing].to_hex(16)),
                Block::Default       => panic!("default block:\n{}", &input[..advancing].to_hex(16)),
                Block::Avih(h)       => {
                    println!("got main AVI header: {:?}", h);
                    (advancing, State::Blocks(ctx))
                },
                Block::List(size, l) => {
                    match ctx.level {
                        List::Nil => (advancing,
                            State::Blocks(Context {
                                file_size: ctx.file_size,
                                stream_offset: ctx.stream_offset,
                                level: List::Node(ctx.stream_offset + size, l, Box::new(List::Nil))
                            })),
                        List::Node(sz, _, _) => {
                            if sz < ctx.stream_offset + size {
                                // the new list would be larger than the parent one
                                println!("the new list would be larger ({}) than the parent one ({})",
                                    ctx.stream_offset + size, sz);
                                (advancing, State::Error)
                            } else {
                                ctx.level = List::Node(ctx.stream_offset + size, l, Box::new(ctx.level));
                                (advancing,
                                State::Blocks(ctx))
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
