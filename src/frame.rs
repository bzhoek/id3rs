use nom::bits::streaming::tag;
use nom::bits::{bits, streaming::take};
use nom::bytes::streaming::take_until;
use nom::{error, number, AsBytes, IResult};

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

fn bitrate_to_kbps(version: &Version, layer: &Layer, bitrate: u8) -> u32 {
  match (version, layer) {
    (Version::Version1, Layer::Layer3) => match bitrate {
      0b0001 => 32,
      0b0010 => 40,
      0b0011 => 48,
      0b0100 => 56,
      0b0101 => 64,
      0b0110 => 80,
      0b0111 => 96,
      0b1000 => 112,
      0b1001 => 128,
      0b1010 => 160,
      0b1011 => 192,
      0b1100 => 224,
      0b1101 => 256,
      0b1110 => 320,
      _ => 0,
    },
    (_, _) => 0,
  }
}

fn sampling_to_hz(version: &Version, sampling: u8) -> u32 {
  match version {
    Version::Version1 => match sampling {
      0b0000 => 44100,
      0b0001 => 48000,
      0b0010 => 32000,
      _ => 0,
    },
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
  pub frequency: u32,
  pub padding: u8,
}

impl FrameHeader {
  pub fn frame_size(&self) -> u32 {
    frame_size(&self.layer, self.bitrate, self.frequency, self.padding)
  }
}

pub fn frame_size(layer: &Layer, bitrate: u32, frequency: u32, padding: u8) -> u32 {
  match layer {
    Layer::Layer1 => 12 * bitrate / frequency * 4,
    Layer::Layer2 | Layer::Layer3 => 144000 * bitrate / frequency + padding as u32,
    Layer::Reserved => panic!("Invalid layer"),
  }
}

// http://id3lib.sourceforge.net/id3/mp3frame.html and http://www.mp3-tech.org/programmer/frame_header.html
#[allow(dead_code, unused)]
pub fn frame_header(input: &[u8]) -> IResult<&[u8], FrameHeader> {
  let (input, _) = take_until(b"\xff".as_bytes())(input)?;
  let (_input, word) = number::streaming::be_u16(input)?;
  println!("{:b}", word);
  if (word & 0xffe0) != 0xffe0 {
    return Err(nom::Err::Error(error::Error::new(input, error::ErrorKind::Tag)));
  }

  let (input, _) = nom::bytes::streaming::take(1u32)(input)?; // skip 0xff
  let (input, (version_u8, layer_u8, crc)) = bits(frame_header_layer)(input)?;
  let (input, (bitrate_u8, sampling_u8, padding, private)) = bits(frame_header_bitrate)(input)?;
  let (input, (channel, mode, copyright, original, emphasis)) = bits(frame_header_mode)(input)?;
  println!("input size {:?}", input.len());

  // let (input, frame) = nom::bits::complete::tag(0b111, 3usize)(input)?;
  // let (_input, (bitrate, frequency, padding,private))
  //   = bits(tuple((take(4usize), take(2usize), take(1usize), take(1usize))))(input)?;

  let version = Version::from(version_u8);
  let layer = Layer::from(layer_u8);
  let bitrate = bitrate_to_kbps(&version, &layer, bitrate_u8);
  let frequency = sampling_to_hz(&version, sampling_u8);
  let size = frame_size(&layer, bitrate, frequency, padding);
  let (input, data) = nom::bytes::streaming::take(size - 4)(input)?;
  let frame = FrameHeader {
    version,
    layer,
    crc: Protection::from(crc),
    bitrate,
    frequency,
    padding,
  };

  Ok((input, frame))
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
