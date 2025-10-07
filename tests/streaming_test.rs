use id3rs::frame::{frame_header, frame_sync, FrameHeader};
use log::LevelFilter;
use nom::error::ErrorKind;
use nom::Err::Incomplete;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Seek};

const CHUNK_SIZE: usize = 1024;

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

impl Iterator for FrameParser {
  type Item = FrameHeader;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      let remaining = match frame_sync(&self.buffer) {
        Ok((remaining, _)) => remaining,
        Err(_) => {
          match self.read_more() {
            Ok(_) => continue,
            Err(_) => return None, // EOF
          }
        }
      };

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
  }
}

#[cfg(test)]
#[ctor::ctor]
fn init() {
  let _ = env_logger::builder().is_test(true).filter_level(LevelFilter::Debug).try_init();
}

#[cfg(test)]
mod tests {
  use id3rs::frame::FrameHeader;
  use std::fs::File;
  use std::io::Write;

  #[test]
  fn test_writing() {
    let mut file = File::create("frames.mp3").unwrap();
    let file_iter = crate::FrameParser::new("samples/4tink.mp3").unwrap();
    for header in file_iter {
      file.write_all(&*header.data).unwrap();
    }
    file.flush().unwrap();
  }

  #[test]
  fn test_iterator() {
    let file_iter = crate::FrameParser::new("samples/4tink.mp3").unwrap();
    let frames: Vec<_> = file_iter.collect();
    assert_eq!(26, frames.len());
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
