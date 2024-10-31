pub struct Mp3Header<'a> {
  pub bytes: &'a [u8; 4],
}

impl Mp3Header<'_> {
  pub fn new(bytes: &[u8; 4]) -> Mp3Header {
    Mp3Header { bytes }
  }

  pub fn version(&self) -> u8 {
    (self.bytes[1] & 0b00011000) >> 3
  }

  pub fn layer(&self) -> u8 {
    (self.bytes[1] & 0b00000110) >> 1
  }

  pub fn bitrate(&self) -> u8 {
    (self.bytes[2] & 0b11110000) >> 4
  }

  pub fn frequency(&self) -> u8 {
    (self.bytes[2] & 0b00001100) >> 2
  }

  pub fn kbit(&self) -> u32 {
    match (self.version(), self.layer()) {
      (0b11, 0b01) => {
        match self.bitrate() {
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
          _ => unreachable!("Invalid bitrate"),
        }
      }
      _ => unreachable!("Invalid version or layer"),
    }
  }

  pub fn len(&self) -> u32 {
    let hz = match self.frequency() {
      0b00 => 44100,
      0b01 => 48000,
      0b10 => 32000,
      _ => unreachable!("Invalid frequency"),
    };
    144000 * self.kbit() / hz
  }
}

#[cfg(test)]
mod tests {
  use id3rs::frame::{frame_header, FrameHeader, Layer, Protection, Version};
  use crate::Mp3Header;

  #[test]
  fn find_frame_header() {
    let buffer = include_bytes!("../samples/4tink.mp3");
    let (position, frame) = frame_header(&buffer[1114..]).ok().unwrap();
    assert_eq!(buffer.len() - position.len(), 1128);
    println!("{:?}", frame);
    assert_eq!(frame, FrameHeader {
      version: Version::Version1,
      layer: Layer::Layer3,
      crc: Protection::Unprotected,
      bitrate: 160,
      frequency: 44100,
    });
  }

  #[test]
  fn parse_mp3_header() {
    let bytes = b"\xFF\xFB\x94\x44";

    let header = Mp3Header::new(bytes);
    assert_eq!(header.len(), 384)
  }

  #[test]
  fn parse_frame_header() {
    let bytes = b"\xFF\xFB\x94\x44";

    let (position, frame) = frame_header(bytes).ok().unwrap();
    println!("{:?}", frame);
    assert_eq!(frame, FrameHeader {
      version: Version::Version1,
      layer: Layer::Layer3,
      crc: Protection::Unprotected,
      bitrate: 128,
      frequency: 48000,
    });
    assert_eq!(frame.frame_size(), 384)
  }
}