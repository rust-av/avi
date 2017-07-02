use nom::{le_u32,HexDisplay,IResult,Offset};
use parser::{self,block,bitmap_info_header,header,strf,AVIStreamHeader,BitmapInfoHeader,Block,FccType};

#[derive(Debug,Clone,PartialEq)]
pub enum State {
    Initial,
    Error,
    Blocks(Context),
    VideoIndexStream(Context,VideoIndexState),
    AudioIndexStream(Context),
    SubtitleIndexStream(Context),
}

#[derive(Debug,Clone,PartialEq)]
pub enum VideoIndexState {
    Initial(AVIStreamHeader),
    BMP(AVIStreamHeader,BitmapInfoHeader),
    Index(AVIStreamHeader,BitmapInfoHeader),
    End(AVIStreamHeader,BitmapInfoHeader),
    Error,
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
    video:         Option<VideoContext>,
}

#[derive(Debug,Clone,PartialEq)]
pub struct VideoContext {
    stream: AVIStreamHeader,
    bitmap: BitmapInfoHeader,
}

pub fn advance(state: State, input: &[u8]) -> (usize, State) {
    match state {
        State::Initial => parse_initial(input),
        State::Blocks(context) => parse_blocks(input, context),
        State::VideoIndexStream(context, index_state) => parse_video_index_stream(input, context, index_state),
        State::AudioIndexStream(context) => parse_audio_index_stream(input, context),
        State::SubtitleIndexStream(context) => parse_subtitle_index_stream(input, context),
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
            level: List::Nil,
            video: None,
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
                    println!("got main AVI header: {:?}\n", h);
                    (advancing, State::Blocks(ctx))
                },
                Block::Strh(h)       => {
                    println!("got AVI stream header: {:?}\n", h);
                    match h.fcc_type {
                        FccType::Video    => {
                            if ctx.video.is_none() {
                                (advancing, State::VideoIndexStream(
                                    ctx,
                                    VideoIndexState::Initial(h)
                                ))
                            } else {
                                (0, State::Error)
                            }
                        },
                        FccType::Audio    => (advancing, State::AudioIndexStream(ctx)),
                        FccType::Subtitle => (advancing, State::SubtitleIndexStream(ctx)),
                    }
                },
                Block::List(size, l) => {
                    match ctx.level {
                        List::Nil => (advancing,
                            State::Blocks(Context {
                                file_size: ctx.file_size,
                                stream_offset: ctx.stream_offset,
                                level: List::Node(ctx.stream_offset + size, l, Box::new(List::Nil)),
                                video: None,
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

pub fn parse_video_index_stream(input: &[u8], mut ctx: Context, mut state: VideoIndexState) -> (usize, State) {
    match state {
        VideoIndexState::Initial(header) => match strf(input) {
            IResult::Error(_)        => (0, State::Error),
            IResult::Incomplete(_)   => (0, State::VideoIndexStream(ctx, VideoIndexState::Initial(header))),
            IResult::Done(i, bmp_header) => {
                println!("got a bitmap info header: {:?}\n", bmp_header);
                let advancing = input.offset(i);
                ctx.stream_offset += advancing;
                (advancing, State::VideoIndexStream(ctx, VideoIndexState::BMP(header, bmp_header)))
            },
        },
        VideoIndexState::BMP(header, bmp_header) => {
            match tuple!(input, tag!(b"JUNK"), le_u32) {
                IResult::Error(_)         => (0, State::Error),
                IResult::Incomplete(_)    => (0, State::VideoIndexStream(ctx, VideoIndexState::BMP(header, bmp_header))),
                IResult::Done(i, (_, sz)) => {
                    let advancing = input.offset(i) + sz as usize;
                    ctx.stream_offset += advancing;
                    ctx.video = Some(VideoContext {
                        stream: header,
                        bitmap: bmp_header,
                    });
                    (advancing, State::Blocks(ctx))
                }

            }
        }
        _ => unimplemented!()
    }
}

pub fn parse_audio_index_stream(input: &[u8], mut ctx: Context) -> (usize, State) {
    unimplemented!()
}

pub fn parse_subtitle_index_stream(input: &[u8], mut ctx: Context) -> (usize, State) {
    unimplemented!()
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
        let     data      = drop;

        loop {
            if offset > data.len() {
                println!("file ended");
                break;
            }

            println!("\nwill parse:\n{}\n", &data[offset..offset+512].to_hex(16));
            let state = opt_state.take().expect("should not be none here");
            let (mv, state) = advance(state, &data[offset..]);

            println!("state is: {:?} (advancing {})", state, mv);
            offset += mv;

            opt_state = Some(state);
        }
    }

}
