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
  Crc,
  Unprotected,
}

impl From<u8> for Protection {
  fn from(version: u8) -> Protection {
    match version {
      0 => Protection::Crc,
      _ => Protection::Unprotected,
    }
  }
}

#[derive(Debug, PartialEq)]
pub struct FrameHeader {
  version: Version,
  layer: Layer,
  crc: Protection,
}


#[allow(dead_code)]
fn do_everything_bits(i: (&[u8], usize)) -> IResult<(&[u8], usize), (u8, u8, u8)> {
  let (i, _) = tag(0b1111, 4usize)(i)?;
  let (i, id) = take(1usize)(i)?;
  let (i, layer) = take(2usize)(i)?;
  let (i, protected) = take(1usize)(i)?;
  Ok((i, (id, layer, protected)))
}

#[allow(dead_code)]
fn frame_header(input: &[u8]) -> IResult<&[u8], FrameHeader> {
  let (input, _) = take_until(b"\xff".as_bytes())(input)?;
  let (input, _) = nom::bytes::streaming::take(1u32)(input)?;
  let (input, (version, layer, crc)) = bits(do_everything_bits)(input)?;
  println!("{:?}", input.len());
  let frame = FrameHeader { version: Version::from(version), layer: Layer::from(layer), crc: Protection::from(crc) };
  Ok((input, frame))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn find_frame_header() {
    let buffer = include_bytes!("../samples/4tink.mp3");
    let (position, frame) = frame_header(&buffer[1114..]).ok().unwrap();
    assert_eq!(buffer.len() - position.len(), 1126);
    println!("{:?}", frame);
    assert_eq!(frame, FrameHeader {
      version: Version::Reserved,
      layer: Layer::Layer3,
      crc: Protection::Unprotected,
    });
  }
}