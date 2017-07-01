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
        let data = header(&verona[..12]);
        println!("data: {:?}", data);
        panic!();
    }
}
