use nom::{IResult,Offset};
use parser::header;

#[derive(Debug,Clone,PartialEq)]
pub enum State {
    Initial,
    Error,
    Header(usize),
}

pub fn advance(state: State, input: &[u8]) -> (usize, State) {
    match state {
        State::Initial => parse_initial(input),
        _              => panic!("unimplemented state"),



    }
}

pub fn parse_initial(input: &[u8]) -> (usize, State) {
    match header(input) {
        IResult::Error(_)        => (0, State::Error),
        IResult::Incomplete(_)   => (0, State::Initial),
        IResult::Done(i, header) => (input.offset(i), State::Header(header.file_size as usize)),
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

            let state = opt_state.take().expect("should not be none here");
            let (mv, state) = advance(state, &drop[offset..]);
            offset += mv;

            opt_state = Some(state);
        }
    }

}
