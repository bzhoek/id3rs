use nom::{AsBytes, IResult};
use nom::bits::{bits, streaming::take};
use nom::bits::streaming::tag;
use nom::bytes::streaming::take_until;

#[derive(Debug, PartialEq)]
enum Version {
  Version25,
  Version2,
  Version1,
  Reserved,
}

impl From<u8> for Version {
  fn from(version: u8) -> Version {
    match version {
      0b00 => Version::Version25,
      0b10 => Version::Version2,
      0b11 => Version::Version1,
      _ => Version::Reserved,
    }
  }
}

#[derive(Debug, PartialEq)]
enum Layer {
  Layer1,
  Layer2,
  Layer3,
  Reserved,
}

impl From<u8> for Layer {
  fn from(version: u8) -> Layer {
    match version {
      0b01 => Layer::Layer3,
      0b10 => Layer::Layer2,
      0b11 => Layer::Layer1,
      _ => Layer::Reserved,
    }
  }
}

#[derive(Debug, PartialEq)]
enum Protection {
  CRC,
  Unprotected,
}

impl From<u8> for Protection {
  fn from(version: u8) -> Protection {
    match version {
      0 => Protection::CRC,
      _ => Protection::Unprotected,
    }
  }
}

#[derive(Debug, PartialEq)]
pub struct Frame {
  version: Version,
  layer: Layer,
  crc: Protection,
}


fn do_everything_bits(i: (&[u8], usize)) -> IResult<(&[u8], usize), (u8, u8, u8, u8)> {
  let (i, a) = tag(0b111, 3usize)(i)?;
  let (i, b) = take(2usize)(i)?;
  let (i, c) = take(2usize)(i)?;
  let (i, d) = take(1usize)(i)?;
  Ok((i, (a, b, c, d)))
}

fn file_header(input: &[u8]) -> IResult<&[u8], Frame> {
  let (input, _) = take_until(b"\xff".as_bytes())(input)?;
  println!("{:?}", input.len());
  let (input, (_, version, layer, crc)) = bits(do_everything_bits)(input)?;
  println!("{:?}", input.len());
  // bits(header)(in)
  // let (input, frame) = nom::bits::complete::tag(0b111, 3usize)(input)?;
  // let (_input, (sync, version, layer, crc))
  //   = bits(tuple((take(3usize), take(2usize), take(2usize), take(1usize))))(input)?;
  Ok((input, Frame { version: Version::from(version), layer: Layer::from(layer), crc: Protection::from(crc) }))
}

#[cfg(test)]
mod tests {
  use crate::mp3::{file_header, Frame, Layer, Protection, Version};

  #[test]
  fn find_signature() {
    let buffer = include_bytes!("../4bleak.mp3");
    println!("{}", buffer.len()); // 12884121 - 12841795
    let (position, frame) = file_header(&buffer[41303..]).ok().unwrap();
    assert_eq!(buffer.len() - position.len(), 42327);
    println!("{:?}", frame);
    assert_eq!(frame, Frame {
      version: Version::Version1,
      layer: Layer::Layer1,
      crc: Protection::Unprotected,
    });
  }
}