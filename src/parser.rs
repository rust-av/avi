use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    combinator::{map, verify},
    number::complete::{le_i16, le_i32, le_u16, le_u32},
    sequence::tuple,
    IResult,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Header<'a> {
    magic1: &'a [u8],
    pub file_size: u32,
    magic2: &'a [u8],
}

pub fn header(input: &[u8]) -> IResult<&[u8], Header> {
    map(
        alt((
            tuple((
                tag(b"RIFF"),
                le_u32,
                alt((tag(b"AVI "), tag(b"AVIX"), tag(b"AVI\x19"), tag(b"AMV "))),
            )),
            tuple((tag(b"ON2 "), le_u32, tag(b"ON2f"))),
        )),
        |(magic1, file_size, magic2)| Header {
            magic1,
            file_size,
            magic2,
        },
    )(input)
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockHeader<'a> {
    tag: &'a [u8],
    size: u32,
}

pub fn block_header<'a>(input: &'a [u8]) -> IResult<&'a [u8], BlockHeader<'a>> {
    map(tuple((take(4usize), le_u32)), |(tag, size)| BlockHeader {
        tag,
        size,
    })(input)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    List(usize, List),
    Avih(MainAVIHeader),
    Strh(AVIStreamHeader),
    Unimplemented,
    Default,
}

#[derive(Debug, Clone, PartialEq)]
pub enum List {
    Hdrl,
    Movi(usize),
    Default,
    Unknown(Vec<u8>),
}

pub fn list(
    input: &[u8],
    stream_offset: usize,
    file_size: u32,
    list_size: u32,
) -> IResult<&[u8], List> {
    map(take(4usize), |val: &[u8]| match val {
        b"INFO" => List::Default,
        b"ncdt" => List::Default,
        b"movi" => {
            if list_size != 0 {
                let offset = stream_offset +
                4 + // tag  (4 bytes)
                4; // size (4 bytes)

                // FIXME: check for overflow
                List::Movi(offset + list_size as usize + (list_size & 1) as usize)
            } else {
                List::Movi(file_size as usize)
            }
        }
        b"hdrl" => List::Hdrl,
        a => List::Unknown(a.to_owned()),
    })(input)
}

/// block()
///
/// stream_offset is the offset corresponding to the position of `input` from the beginning of the stream
pub fn block(input: &[u8], stream_offset: usize, file_size: u32) -> IResult<&[u8], Block> {
    tuple((take(4usize), le_u32))(input).and_then(|(i, (tag, size))| match tag {
        b"LIST" => list(i, stream_offset, file_size, size)
            .and_then(|(i, l)| Ok((i, Block::List(size as usize, l)))),
        b"IDIT" => Ok((i, Block::Unimplemented)),
        b"dmlh" => Ok((i, Block::Unimplemented)),
        b"amvh" => Ok((i, Block::Unimplemented)),
        b"avih" => map(avih, |h| Block::Avih(h))(i),
        b"strh" => map(strh, |h| Block::Strh(h))(i),
        b"strf" => Ok((i, Block::Unimplemented)),
        b"indx" => Ok((i, Block::Unimplemented)),
        b"vprp" => Ok((i, Block::Unimplemented)),
        b"strn" => Ok((i, Block::Unimplemented)),
        _ => Ok((i, Block::Default)),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct MainAVIHeader {
    microsec_per_frame: u32,
    max_bytes_per_sec: u32,
    padding_granularity: u32,
    flags: u32,
    total_frames: u32,
    initial_frames: u32,
    streams: u32,
    suggested_buffer_size: u32,
    width: u32,
    height: u32,
}

pub fn avih(input: &[u8]) -> IResult<&[u8], MainAVIHeader> {
    map(
        tuple((
            le_u32,
            le_u32,
            le_u32,
            le_u32,
            le_u32,
            le_u32,
            le_u32,
            le_u32,
            le_u32,
            le_u32,
            take(16usize),
        )),
        |t| MainAVIHeader {
            microsec_per_frame: t.0,
            max_bytes_per_sec: t.1,
            padding_granularity: t.2,
            flags: t.3,
            total_frames: t.4,
            initial_frames: t.5,
            streams: t.6,
            suggested_buffer_size: t.7,
            width: t.8,
            height: t.9,
        },
    )(input)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rect {
    left: i16,
    top: i16,
    right: i16,
    bottom: i16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AVIStreamHeader {
    pub fcc_type: FccType,
    fcc_handler: u32,
    flags: u32,
    priority: u16,
    language: u16,
    initial_frames: u32,
    scale: u32,
    rate: u32,
    start: u32,
    length: u32,
    suggested_buffer_size: u32,
    quality: u32,
    sample_size: u32,
    frame: Rect,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FccType {
    Video,
    Audio,
    Subtitle,
}

pub fn strh(input: &[u8]) -> IResult<&[u8], AVIStreamHeader> {
    map(
        tuple((
            fcc_type, le_u32, le_u32, le_u16, le_u16, le_u32, le_u32, le_u32, le_u32, le_u32,
            le_u32, le_u32, le_u32, le_i16, le_i16, le_i16, le_i16,
        )),
        |t| AVIStreamHeader {
            fcc_type: t.0,
            fcc_handler: t.1,
            flags: t.2,
            priority: t.3,
            language: t.4,
            initial_frames: t.5,
            scale: t.6,
            rate: t.7,
            start: t.8,
            length: t.9,
            suggested_buffer_size: t.10,
            quality: t.11,
            sample_size: t.12,
            frame: Rect {
                left: t.13,
                top: t.14,
                right: t.15,
                bottom: t.16,
            },
        },
    )(input)
}

pub fn fcc_type(input: &[u8]) -> IResult<&[u8], FccType> {
    map(take(4usize), |val: &[u8]| match val {
        b"vids" => FccType::Video,
        b"auds" => FccType::Audio,
        b"txts" => FccType::Subtitle,
        _ => unreachable!(),
    })(input)
}

pub fn strf(input: &[u8]) -> IResult<&[u8], BitmapInfoHeader> {
    map(
        tuple((
            tag(b"strf"),
            verify(le_u32, |val| *val == 40),
            bitmap_info_header,
        )),
        |t| t.2,
    )(input)
}

/// as seen on https://msdn.microsoft.com/en-us/library/windows/desktop/dd183376(v=vs.85).aspx
#[derive(Debug, Clone, PartialEq)]
pub struct BitmapInfoHeader {
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bit_count: u16,
    compression: u32,
    size_image: u32,
    xpels_per_meter: i32,
    ypels_per_meter: i32,
    clr_used: u32,
    clr_important: u32,
}

pub fn bitmap_info_header(input: &[u8]) -> IResult<&[u8], BitmapInfoHeader> {
    map(
        tuple((
            le_u32, le_i32, le_i32, le_u16, le_u16, le_u32, le_u32, le_i32, le_i32, le_u32, le_u32,
        )),
        |t| BitmapInfoHeader {
            size: t.0,
            width: t.1,
            height: t.2,
            planes: t.3,
            bit_count: t.4,
            compression: t.5,
            size_image: t.6,
            xpels_per_meter: t.7,
            ypels_per_meter: t.8,
            clr_used: t.9,
            clr_important: t.10,
        },
    )(input)
}
#[cfg(test)]
#[allow(non_upper_case_globals)]
mod tests {
    use nom::HexDisplay;

    use super::*;

    const drop: &[u8] = include_bytes!("../assets/drop.avi");
    const verona: &[u8] = include_bytes!("../assets/verona60avi56k.avi");

    #[test]
    fn parse_header() {
        let data = header(&drop[..12]);
        println!("data: {:?}", data);
        assert_eq!(
            data,
            Ok((
                &b""[..],
                Header {
                    magic1: b"RIFF",
                    file_size: 675628,
                    magic2: b"AVI ",
                }
            ))
        );

        let data = header(&verona[..12]);
        println!("data: {:?}", data);
        assert_eq!(
            data,
            Ok((
                &b""[..],
                Header {
                    magic1: b"RIFF",
                    file_size: 1926660,
                    magic2: b"AVI ",
                }
            ))
        );
    }

    #[test]
    fn parse_block_header() {
        println!("block:\n{}", &drop[12..200].to_hex(16));
        let data = block_header(&drop[12..20]);
        println!("data: {:?}", data);
        assert_eq!(
            data,
            Ok((
                &b""[..],
                BlockHeader {
                    tag: b"LIST",
                    size: 192,
                }
            ))
        );
        let data = block_header(&verona[12..20]);
        println!("data: {:?}", data);
        assert_eq!(
            data,
            Ok((
                &b""[..],
                BlockHeader {
                    tag: b"LIST",
                    size: 370,
                }
            ))
        );
    }

    #[test]
    fn parse_block() {
        println!("block:\n{}", &drop[12..24].to_hex(16));
        let data = block(&drop[12..24], 12, 675628);
        println!("data: {:?}", data);
        assert_eq!(
            data,
            Ok((
                &b""[..],
                Block::List(
                    192,
                    List::Unknown(vec!('h' as u8, 'd' as u8, 'r' as u8, 'l' as u8))
                )
            ))
        );
        let data = block(&verona[12..24], 12, 1926660);
        println!("data: {:?}", data);
        assert_eq!(
            data,
            Ok((
                &b""[..],
                Block::List(
                    370,
                    List::Unknown(vec!('h' as u8, 'd' as u8, 'r' as u8, 'l' as u8))
                )
            ))
        );
    }

    #[test]
    fn parse_block2() {
        println!("block:\n{}", &drop[112..120].to_hex(16));
        let data = block(&drop[112..120], 112, 675628);
        println!("data: {:?}", data);
        assert_eq!(data, Ok((&b""[..], Block::Default)));
        let data = block(&verona[382..398], 382, 1926660);
        println!("data: {:?}", data);
        assert_eq!(data, Ok((&b""[..], Block::Default)));
    }
}
