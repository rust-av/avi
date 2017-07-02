use nom::{IResult,le_i16,le_u16,le_u32};

#[derive(Debug,Clone,PartialEq)]
pub struct Header<'a> {
    magic1:    &'a [u8],
    pub file_size: u32,
    magic2:    &'a [u8],
}

pub fn header(input: &[u8]) -> IResult<&[u8], Header> {
    map!(input,
        alt!(
          tuple!(
              tag!(b"RIFF"),
              le_u32,
              alt!(tag!(b"AVI ") | tag!(b"AVIX") | tag!(b"AVI\x19") | tag!(b"AMV "))
          )
        | tuple!(
              tag!(b"ON2 "),
              le_u32,
              tag!(b"ON2f")
          )
        ),
        |(magic1, file_size, magic2)| {
            Header {
                magic1,
                file_size,
                magic2,
            }
        }
    )
}

#[derive(Debug,Clone,PartialEq)]
pub struct BlockHeader<'a> {
    tag:  &'a [u8],
    size: u32,
}

named!(pub block_header<BlockHeader>,
    do_parse!(
        tag:  take!(4) >>
        size: le_u32   >>
        (BlockHeader {
            tag,
          size,
        })
    )
);

#[derive(Debug,Clone,PartialEq)]
pub enum Block {
    List(usize, List),
    Avih(MainAVIHeader),
    Strh(AVIStreamHeader),
    Unimplemented,
    Default,
}

#[derive(Debug,Clone,PartialEq)]
pub enum List {
    Hdrl,
    Movi(usize),
    Default,
    Unknown(Vec<u8>),
}

pub fn list(input: &[u8], stream_offset: usize, file_size: u32, list_size: u32) -> IResult<&[u8], List> {
    switch!(input, take!(4),
        b"INFO" => value!(List::Default) |
        b"ncdt" => value!(List::Default) |
        b"movi" => value!({
          if list_size != 0 {
              let offset = stream_offset +
                4 + // tag  (4 bytes)
                4; // size (4 bytes)

              // FIXME: check for overflow
              List::Movi(offset + list_size as usize + (list_size & 1) as usize)
          } else {
              List::Movi(file_size as usize)
          }
        })                               |
        b"hdrl" => value!(List::Hdrl)    |
        a       => value!(List::Unknown(a.to_owned()))
    )
}

/// block()
///
/// stream_offset is the offset corresponding to the position of `input` from the beginning of the stream
pub fn block(input: &[u8], stream_offset: usize, file_size: u32) -> IResult<&[u8], Block> {
    do_parse!(input,
        tag:   take!(4) >>
        size:  le_u32   >>
        block: switch!(value!(tag),
          b"LIST" => map!(call!(list, stream_offset, file_size, size), |l| Block::List(size as usize, l)) |
          b"IDIT" => value!(Block::Unimplemented) |
          b"dmlh" => value!(Block::Unimplemented) |
          b"amvh" => value!(Block::Unimplemented) |
          b"avih" => map!(avih, |h| Block::Avih(h)) |
          b"strh" => map!(strh, |h| Block::Strh(h)) |
          b"strf" => value!(Block::Unimplemented) |
          b"indx" => value!(Block::Unimplemented) |
          b"vprp" => value!(Block::Unimplemented) |
          b"strn" => value!(Block::Unimplemented) |
          _       => value!(Block::Default)
        )  >>
        (block)

    )
}

#[derive(Debug,Clone,PartialEq)]
pub struct MainAVIHeader {
    microsec_per_frame:    u32,
    max_bytes_per_sec:     u32,
    padding_granularity:   u32,
    flags:                 u32,
    total_frames:          u32,
    initial_frames:        u32,
    streams:               u32,
    suggested_buffer_size: u32,
    width:                 u32,
    height:                u32,
}

pub fn avih(input: &[u8]) -> IResult<&[u8], MainAVIHeader> {
    do_parse!(input,
        microsec_per_frame:    le_u32 >>
        max_bytes_per_sec:     le_u32 >>
        padding_granularity:   le_u32 >>
        flags:                 le_u32 >>
        total_frames:          le_u32 >>
        initial_frames:        le_u32 >>
        streams:               le_u32 >>
        suggested_buffer_size: le_u32 >>
        width:                 le_u32 >>
        height:                le_u32 >>
                               take!(16) >>
        (MainAVIHeader {
            microsec_per_frame,
            max_bytes_per_sec,
            padding_granularity,
            flags,
            total_frames,
            initial_frames,
            streams,
            suggested_buffer_size,
            width,
            height,
        })
    )
}

#[derive(Debug,Clone,PartialEq)]
pub struct Rect {
    left:   i16,
    top:    i16,
    right:  i16,
    bottom: i16,
}

#[derive(Debug,Clone,PartialEq)]
pub struct AVIStreamHeader {
    fcc_type:               u32,
    fcc_handler:            u32,
    flags:                  u32,
    priority:               u16,
    language:               u16,
    initial_frames:         u32,
    scale:                  u32,
    rate:                   u32,
    start:                  u32,
    length:                 u32,
    suggested_buffer_size:  u32,
    quality:                u32,
    sample_size:            u32,
    frame:                  Rect,
}

pub fn strh(input: &[u8]) -> IResult<&[u8], AVIStreamHeader> {
    do_parse!(input,
        fcc_type:               le_u32 >>
        fcc_handler:            le_u32 >>
        flags:                  le_u32 >>
        priority:               le_u16 >>
        language:               le_u16 >>
        initial_frames:         le_u32 >>
        scale:                  le_u32 >>
        rate:                   le_u32 >>
        start:                  le_u32 >>
        length:                 le_u32 >>
        suggested_buffer_size:  le_u32 >>
        quality:                le_u32 >>
        sample_size:            le_u32 >>
        left:                   le_i16 >>
        top:                    le_i16 >>
        right:                  le_i16 >>
        bottom:                 le_i16 >>
        (AVIStreamHeader {
          fcc_type,
          fcc_handler,
          flags,
          priority,
          language,
          initial_frames,
          scale,
          rate,
          start,
          length,
          suggested_buffer_size,
          quality,
          sample_size,
          frame: Rect {
            left, top, right, bottom
          },
        })
    )
}

#[cfg(test)]
mod tests {
    use nom::{HexDisplay,IResult};
    use super::*;

    const drop   : &'static [u8] = include_bytes!("../assets/drop.avi");
    const verona : &'static [u8] = include_bytes!("../assets/verona60avi56k.avi");

    #[test]
    fn parse_header() {
        let data = header(&drop[..12]);
        println!("data: {:?}", data);
        assert_eq!(data,
            IResult::Done(
                &b""[..],
                Header {
                    magic1:    b"RIFF",
                    file_size: 675628,
                    magic2:    b"AVI ",
            })
        );

        let data = header(&verona[..12]);
        println!("data: {:?}", data);
        assert_eq!(data,
            IResult::Done(
                &b""[..],
                Header {
                    magic1:    b"RIFF",
                    file_size: 1926660,
                    magic2:    b"AVI ",
            })
        );
    }

    #[test]
    fn parse_block_header() {
        println!("block:\n{}", &drop[12..200].to_hex(16));
        let data = block_header(&drop[12..20]);
        println!("data: {:?}", data);
        assert_eq!(data,
            IResult::Done(
                &b""[..],
                BlockHeader {
                    tag: b"LIST",
                    size: 192,
            })
        );
        let data = block_header(&verona[12..20]);
        println!("data: {:?}", data);
        assert_eq!(data,
            IResult::Done(
                &b""[..],
                BlockHeader {
                    tag: b"LIST",
                    size: 370,
            })
        );
    }

    #[test]
    fn parse_block() {
        println!("block:\n{}", &drop[12..24].to_hex(16));
        let data = block(&drop[12..24], 12, 675628);
        println!("data: {:?}", data);
        assert_eq!(data,
            IResult::Done(
                &b""[..],
                Block::List(192, List::Unknown(vec!('h' as u8, 'd' as u8, 'r' as u8, 'l' as u8)))
            )
        );
        let data = block(&verona[12..24], 12, 1926660);
        println!("data: {:?}", data);
        assert_eq!(data,
            IResult::Done(
                &b""[..],
                Block::List(370, List::Unknown(vec!('h' as u8, 'd' as u8, 'r' as u8, 'l' as u8)))
            )
        );
    }

    #[test]
    fn parse_block2() {
        println!("block:\n{}", &drop[112..120].to_hex(16));
        let data = block(&drop[112..120], 112, 675628);
        println!("data: {:?}", data);
        assert_eq!(data,
            IResult::Done(
                &b""[..],
                Block::Default
            )
        );
        let data = block(&verona[382..398], 382, 1926660);
        println!("data: {:?}", data);
        assert_eq!(data,
            IResult::Done(
                &b""[..],
                Block::Default
            )
        );
    }
}
