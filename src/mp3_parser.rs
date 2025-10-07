use std::error::Error;
use std::fs::File;
use std::io;
use std::io::{Read, Seek};
use nom::Err::Incomplete;
use nom::error::ErrorKind;
use crate::mp3_frame::{frame_header, frame_sync, FrameHeader};

const CHUNK_SIZE: usize = 1024;

pub struct Mp3FrameParser {
  file: File,
  buffer: Vec<u8>,
  ceiling: usize,
}

impl Mp3FrameParser {
  pub fn new(filename: &str) -> io::Result<Self> {
    let file = File::open(filename)?;
    Ok(Mp3FrameParser {
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

impl Iterator for Mp3FrameParser {
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

