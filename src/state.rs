use std::cmp::{min, Ordering};

use nom::{
    bytes::complete::tag, error::Error, number::complete::le_u32, sequence::tuple, Err, HexDisplay,
    Offset,
};

use crate::parser::{self, block, header, strf, AVIStreamHeader, BitmapInfoHeader, Block, FccType};

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Initial,
    Error,
    Blocks(Context),
    VideoIndexStream(Context, VideoIndexState),
    AudioIndexStream(Context),
    SubtitleIndexStream(Context),
}

#[derive(Debug, Clone, PartialEq)]
pub enum VideoIndexState {
    Initial(AVIStreamHeader),
    BMP(AVIStreamHeader, BitmapInfoHeader),
    Index(AVIStreamHeader, BitmapInfoHeader),
    End(AVIStreamHeader, BitmapInfoHeader),
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct List {
    end_offset: usize,
    current: parser::List,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    file_size: usize,
    stream_offset: usize,
    level: Vec<List>,
    video: Option<VideoContext>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoContext {
    stream: AVIStreamHeader,
    bitmap: BitmapInfoHeader,
}

pub fn advance(state: State, input: &[u8]) -> (usize, State) {
    match state {
        State::Initial => parse_initial(input),
        State::Blocks(context) => parse_blocks(input, context),
        State::VideoIndexStream(mut context, index_state) => {
            match parse_video_index_stream(input, &mut context, index_state) {
                (_, VideoIndexState::Error) => (0, State::Error),
                (advancing, VideoIndexState::End(stream, bitmap)) => {
                    context.stream_offset += advancing;
                    context.video = Some(VideoContext { stream, bitmap });
                    (advancing, State::Blocks(context))
                }
                (advancing, video_state) => {
                    context.stream_offset += advancing;
                    (advancing, State::VideoIndexStream(context, video_state))
                }
            }
        }
        State::AudioIndexStream(context) => parse_audio_index_stream(input, context),
        State::SubtitleIndexStream(context) => parse_subtitle_index_stream(input, context),
        _ => panic!("unimplemented state"),
    }
}

pub fn parse_initial(input: &[u8]) -> (usize, State) {
    match header(input) {
        Err(Err::Error(e)) => {
            println!("got error: {:?}", e);
            (0, State::Error)
        }
        Err(Err::Failure(f)) => {
            println!("got failure: {:?}", f);
            (0, State::Error)
        }
        Err(Err::Incomplete(_)) => (0, State::Initial),
        Ok((i, header)) => (
            input.offset(i),
            State::Blocks(Context {
                file_size: header.file_size as usize,
                stream_offset: input.offset(i),
                level: Vec::new(),
                video: None,
            }),
        ),
    }
}

pub fn unpack_list(input: &[u8], mut ctx: Context) -> (&[u8], Context) {
    loop {
        if ctx.level.is_empty() {
            return (input, ctx);
        } else {
            let end_offset = ctx.level[ctx.level.len() - 1].end_offset;
            match ctx.stream_offset.cmp(&end_offset) {
                Ordering::Less => {
                    return (
                        &input[..min(end_offset - ctx.stream_offset, input.len())],
                        ctx,
                    )
                }
                Ordering::Equal => {
                    ctx.level.pop();
                }
                Ordering::Greater => {
                    panic!("the stream offset should never get farther than the list's end");
                }
            }
        }
    }
}

pub fn parse_blocks(input: &[u8], ctx: Context) -> (usize, State) {
    let (sl, mut ctx) = unpack_list(input, ctx);

    match block(sl, ctx.stream_offset, ctx.file_size as u32) {
        Err(Err::Error(e)) => {
            println!("got error: {:?}", e);
            (0, State::Error)
        }
        Err(Err::Failure(f)) => {
            println!("got failure: {:?}", f);
            (0, State::Error)
        }
        Err(Err::Incomplete(_)) => (0, State::Blocks(ctx)),
        Ok((i, blk)) => {
            let advancing = input.offset(i);
            ctx.stream_offset += advancing;
            match blk {
                Block::Unimplemented => {
                    panic!("unimplemented block:\n{}", &input[..advancing].to_hex(16))
                }
                Block::Default => panic!("default block:\n{}", &input[..advancing].to_hex(16)),
                Block::Avih(h) => {
                    println!("got main AVI header: {:?}\n", h);
                    (advancing, State::Blocks(ctx))
                }
                Block::Strh(h) => {
                    println!("got AVI stream header: {:?}\n", h);
                    match h.fcc_type {
                        FccType::Video => {
                            if ctx.video.is_none() {
                                (
                                    advancing,
                                    State::VideoIndexStream(ctx, VideoIndexState::Initial(h)),
                                )
                            } else {
                                (0, State::Error)
                            }
                        }
                        FccType::Audio => (advancing, State::AudioIndexStream(ctx)),
                        FccType::Subtitle => (advancing, State::SubtitleIndexStream(ctx)),
                    }
                }
                Block::List(size, l) => {
                    if ctx.level.is_empty() {
                        (
                            advancing,
                            State::Blocks(Context {
                                file_size: ctx.file_size,
                                stream_offset: ctx.stream_offset,
                                level: vec![List {
                                    end_offset: ctx.stream_offset + size,
                                    current: l,
                                }],
                                video: None,
                            }),
                        )
                    } else if ctx.level[ctx.level.len() - 1].end_offset < ctx.stream_offset + size {
                        // the new list would be larger than the parent one
                        println!(
                            "the new list would be larger ({}) than the parent one ({})",
                            ctx.stream_offset + size,
                            ctx.level[ctx.level.len() - 1].end_offset
                        );
                        (advancing, State::Error)
                    } else {
                        ctx.level.push(List {
                            end_offset: ctx.stream_offset + size,
                            current: l,
                        });
                        (advancing, State::Blocks(ctx))
                    }
                }
            }
        }
    }
}

pub fn parse_video_index_stream(
    input: &[u8],
    ctx: &mut Context,
    state: VideoIndexState,
) -> (usize, VideoIndexState) {
    match state {
        VideoIndexState::Initial(header) => match strf(input) {
            Err(Err::Error(e)) => {
                println!("got error: {:?}", e);
                (0, VideoIndexState::Error)
            }
            Err(Err::Failure(f)) => {
                println!("got failure: {:?}", f);
                (0, VideoIndexState::Error)
            }
            Err(Err::Incomplete(_)) => (0, VideoIndexState::Initial(header)),
            Ok((i, bmp_header)) => {
                println!("got a bitmap info header: {:?}\n", bmp_header);
                let advancing = input.offset(i);
                (advancing, VideoIndexState::BMP(header, bmp_header))
            }
        },
        VideoIndexState::BMP(header, bmp_header) => {
            match tuple::<_, _, Error<_>, _>((tag(b"JUNK"), le_u32))(input) {
                Err(Err::Error(e)) => {
                    println!("got error: {:?}", e);
                    (0, VideoIndexState::Error)
                }
                Err(Err::Failure(f)) => {
                    println!("got failure: {:?}", f);
                    (0, VideoIndexState::Error)
                }
                Err(Err::Incomplete(_)) => (0, VideoIndexState::BMP(header, bmp_header)),
                Ok((i, (_, sz))) => {
                    loop {
                        if !ctx.level.is_empty() {
                            let end_offset = ctx.level[ctx.level.len() - 1].end_offset;
                            println!(
                                "stream offset == {} end offset == {}",
                                ctx.stream_offset, end_offset
                            );
                            if ctx.stream_offset + 4 == end_offset {
                                let _ = ctx.level.pop();
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    let advancing = input.offset(i) + sz as usize;
                    (advancing, VideoIndexState::End(header, bmp_header))
                }
            }
        }
        _ => unimplemented!(),
    }
}

pub fn parse_audio_index_stream(_input: &[u8], _ctx: Context) -> (usize, State) {
    unimplemented!()
}

pub fn parse_subtitle_index_stream(_input: &[u8], _ctx: Context) -> (usize, State) {
    unimplemented!()
}

#[cfg(test)]
#[allow(non_upper_case_globals)]
mod tests {
    use super::*;

    const drop: &[u8] = include_bytes!("../assets/drop.avi");
    const _verona: &[u8] = include_bytes!("../assets/verona60avi56k.avi");

    #[test]
    fn state_initial() {
        let mut opt_state = Some(State::Initial);
        let mut offset = 0usize;
        let data = drop;

        loop {
            if offset > data.len() {
                println!("file ended");
                break;
            }

            println!(
                "\nwill parse:\n{}\n",
                &data[offset..offset + 512].to_hex(16)
            );
            let state = opt_state.take().expect("should not be none here");
            let (mv, state) = advance(state, &data[offset..]);

            println!("state is: {:?} (advancing {})", state, mv);
            offset += mv;

            opt_state = Some(state);
        }
    }
}
