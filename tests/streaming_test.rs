use id3rs::frame::{frame_header, frame_sync, FrameHeader};
use nom::error::ErrorKind;
use nom::Err::Incomplete;
use nom::{bytes, error, number, IResult};
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Seek};

const CHUNK_SIZE: usize = 1024; // ridiculously small chunk size for example purposes

struct FrameParser {
  file: File,
  buffer: Vec<u8>,
  ceiling: usize,
}

impl FrameParser {
  fn new(filename: &str) -> io::Result<Self> {
    let file = File::open(filename)?;
    Ok(FrameParser {
      file,
      buffer: Vec::new(),
      ceiling: 0,
    })
  }

  fn seek_back(&mut self, delta: i64) {
    self.ceiling -= delta as usize;
    self.file.seek(io::SeekFrom::Current(-delta)).unwrap();
  }

  fn read_more(&mut self) -> Result<(), Box<dyn Error>> {
    let mut buffer: Vec<u8> = vec![0u8; CHUNK_SIZE];
    let len = self.file.read(&mut buffer).expect("Cannot read file");
    if len == 0 {
      Err("EOF")?
    } else {
      self.buffer = buffer;
      self.ceiling += len;
      Ok(())
    }
  }
}

#[allow(dead_code, unused)]
fn parse_mp3_frame(input: &[u8]) -> IResult<&[u8], Vec<u8>> {
  let (input, result) = bytes::streaming::take_till::<_, _, error::Error<_>>(|b| b == 0xff)(input)?;
  let (_input, word) = number::streaming::be_u16(input)?;
  if (word & 0xffe0) != 0xffe0 {
    return Err(nom::Err::Error(error::Error::new(input, error::ErrorKind::Tag)));
  }
  Ok((input, result.to_vec()))
}

impl Iterator for FrameParser {
  type Item = FrameHeader;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      match frame_sync(&self.buffer) {
        Ok((remaining, _)) => {
          self.buffer = remaining.to_vec();
          match frame_header(&self.buffer) {
            Ok((remaining, frame)) => {
              self.buffer = remaining.to_vec();
              return Some(frame);
            }
            Err(Incomplete(_)) => {
              let delta = self.buffer.len() as i64;
              self.seek_back(delta);
              match self.read_more() {
                Ok(_) => continue,
                Err(_) => return None, // EOF
              }
            }
            Err(nom::Err::Error(e)) if e.code == ErrorKind::Tag => {
              println!("offset {} {:?}", self.ceiling - e.input.len(), e);
              let (_, remainder) = e.input.split_at(1);
              self.buffer = remainder.to_vec();
            }
            Err(e) => {
              panic!("Parse error: {}", e);
            }
          }
        }
        Err(_) => {
          match self.read_more() {
            Ok(_) => continue,
            Err(_) => return None, // EOF
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use id3rs::frame::FrameHeader;
  use std::fs::File;
  use std::io::Write;

  #[test]
  fn test_iterator() {
    let file_iter = crate::FrameParser::new("samples/4tink.mp3").unwrap();
    let mut file = File::create("frames.mp3").unwrap();
    // let header = file_iter.next().unwrap();
    for (i, header) in file_iter.enumerate() {
      println!("Parsed header {}: {:?}", i, header);
      file.write_all(&*header.data).unwrap();
    }
    file.flush().unwrap();
    // let size = header.frame_size();
    // assert_eq!(384, size);
    // println!("Parsed line: {:?}", header);
  }

  #[test]
  fn test_layer3_size() {
    let header = FrameHeader {
      version: id3rs::frame::Version::Version1,
      layer: id3rs::frame::Layer::Layer3,
      crc: id3rs::frame::Protection::Unprotected,
      bitrate: 128,
      frequency: 44100,
      padding: 0,
      data: vec![],
    };
    let size = header.frame_size();
    assert_eq!(417, size);
  }
}
