use nom::{AsBytes, IResult};
use nom::bits::{bits, streaming::take};
use nom::bytes::streaming::take_until;
use nom::error::Error;
use nom::sequence::tuple;

#[derive(Debug)]
enum Version {
  Version25,
  Version2,
  Version1,
  Reserved,
}

impl From<u8> for Version {
  fn from(version: u8) -> Version {
    match version {
      0 => Version::Version25,
      2 => Version::Version2,
      3 => Version::Version1,
      _ => Version::Reserved,
    }
  }
}

#[derive(Debug)]
enum Layer {
  Layer1,
  Layer2,
  Layer3,
  Reserved,
}

impl From<u8> for Layer {
  fn from(version: u8) -> Layer {
    match version {
      2 => Layer::Layer3,
      4 => Layer::Layer2,
      _ => Layer::Reserved,
    }
  }
}

#[derive(Debug)]
enum Protection {
  CRC,
  Unprotected,
}

impl From<u8> for Protection {
  fn from(version: u8) -> Protection {
    match version {
      0 => Protection::CRC,
      _ => Protection::CRC,
    }
  }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Frame {
  version: u8,
}

fn file_header(input: &[u8]) -> IResult<&[u8], (u8, u8, u8, u8)> {
  let (input, _) = take_until(b"\xff".as_bytes())(input)?;
  bits::<_, _, Error<(&[u8], usize)>, _, _>
    (tuple((take(3usize), take(2usize), take(2usize), take(1usize))))(input)
}

#[cfg(test)]
mod tests {
  use crate::mp3::{file_header, Frame};

  #[test]
  fn find_signature() {
    let buffer = include_bytes!("../4bleak.mp3");
    let (_, frame) = file_header(buffer).ok().unwrap();
    println!("{:?}", frame);
    // assert_eq!(frame, Frame { version: 0 });
  }
}