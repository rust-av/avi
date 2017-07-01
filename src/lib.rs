#[macro_use] extern crate nom;

use nom::le_u32;

#[derive(Debug,Clone,PartialEq)]
pub struct Header<'a> {
    magic1:    &'a [u8],
    file_size: u32,
    magic2:    &'a [u8],
}

named!(pub header<Header>,
    map!(
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
);

#[derive(Debug,Clone,PartialEq)]
pub struct BlockHeader {
    tag:  u32,
    size: u32,
}

named!(pub block_header<BlockHeader>,
    do_parse!(
        tag:  le_u32 >>
        size: le_u32 >>
        (BlockHeader {
            tag,
          size,
        })
    )
);

#[cfg(test)]
mod tests {
    use nom::IResult;
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
        let data = block_header(&drop[12..20]);
        println!("data: {:?}", data);
        assert_eq!(data,
            IResult::Done(
                &b""[..],
                BlockHeader {
                    tag: 1414744396,
                    size: 192,
            })
        );
        let data = block_header(&verona[12..20]);
        println!("data: {:?}", data);
        assert_eq!(data,
            IResult::Done(
                &b""[..],
                BlockHeader {
                    tag: 1414744396,
                    size: 370,
            })
        );
    }
}
