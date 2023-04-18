use nom::{AsBytes, IResult};
use nom::bits::{bits, streaming::take};
use nom::bits::streaming::tag;
use nom::bytes::streaming::take_until;

#[derive(Debug, PartialEq)]
pub enum Version {
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
pub enum Layer {
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

fn bitrate_to_kbps(bitrate: u8) -> u32 {
  match bitrate {
    0b0001 => 32,
    0b0010 => 40,
    0b0011 => 56,
    0b0100 => 64,
    0b0101 => 80,
    0b0110 => 96,
    0b0111 => 112,
    0b1000 => 128,
    0b1001 => 160,
    0b1010 => 192,
    0b1011 => 224,
    0b1100 => 256,
    0b1110 => 320,
    _ => 0,
  }
}

#[derive(Debug, PartialEq)]
pub enum Protection {
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
  pub version: Version,
  pub layer: Layer,
  pub crc: Protection,
  pub bitrate: u32,
}

fn frame_header_layer(i: (&[u8], usize)) -> IResult<(&[u8], usize), (u8, u8, u8)> {
  let (i, _) = tag(0b111, 3usize)(i)?;
  let (i, id) = take(2usize)(i)?;
  let (i, layer) = take(2usize)(i)?;
  let (i, protected) = take(1usize)(i)?; // by crc
  Ok((i, (id, layer, protected)))
}

type BitrateFlags = (u8, u8, u8, u8);

fn frame_header_bitrate(i: (&[u8], usize)) -> IResult<(&[u8], usize), BitrateFlags> {
  let (i, bitrate) = take(4usize)(i)?;
  let (i, frequency) = take(2usize)(i)?;
  let (i, padding) = take(1usize)(i)?;
  let (i, private) = take(1usize)(i)?;
  Ok((i, (bitrate, frequency, padding, private)))
}

type ModeFlags = (u8, u8, u8, u8, u8);

fn frame_header_mode(i: (&[u8], usize)) -> IResult<(&[u8], usize), ModeFlags> {
  let (i, channel) = take(2usize)(i)?;
  let (i, mode) = take(2usize)(i)?;
  let (i, copyright) = take(1usize)(i)?;
  let (i, original) = take(1usize)(i)?;
  let (i, emphasis) = take(2usize)(i)?;
  Ok((i, (channel, mode, copyright, original, emphasis)))
}

// http://id3lib.sourceforge.net/id3/mp3frame.html and http://www.mp3-tech.org/programmer/frame_header.html
#[allow(dead_code, unused)]
pub fn frame_header(input: &[u8]) -> IResult<&[u8], FrameHeader> {
  let (input, _) = take_until(b"\xff".as_bytes())(input)?;
  let (input, _) = nom::bytes::streaming::take(1u32)(input)?;
  let (input, (version, layer, crc)) = bits(frame_header_layer)(input)?;
  let (input, (bitrate, frequency, padding, private)) = bits(frame_header_bitrate)(input)?;
  let (input, (channel, mode, copyright, original, emphasis)) = bits(frame_header_mode)(input)?;
  println!("{:?}", input.len());

  // let (input, frame) = nom::bits::complete::tag(0b111, 3usize)(input)?;
  // let (_input, (bitrate, frequency, padding,private))
  //   = bits(tuple((take(4usize), take(2usize), take(1usize), take(1usize))))(input)?;

  let frame = FrameHeader {
    version: Version::from(version),
    layer: Layer::from(layer),
    crc: Protection::from(crc),
    bitrate: bitrate_to_kbps(bitrate),
  };
  Ok((input, frame))
}