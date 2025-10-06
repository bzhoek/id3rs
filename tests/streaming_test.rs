use nom::Err::Incomplete;
use nom::{bytes, error, number, IResult, Parser};
use std::error::Error;
use std::fs::File;
use std::io::{self, Read};

const CHUNK_SIZE: usize = 8; // ridiculously small chunk size for example purposes

struct FrameParser {
  file: File,
  buffer: Vec<u8>,
  offset: usize,
}

impl FrameParser {
  fn new(filename: &str) -> io::Result<Self> {
    let file = File::open(filename)?;
    Ok(FrameParser {
      file,
      buffer: Vec::new(),
      offset: 0,
    })
  }

  fn read_more(&mut self) -> Result<(), Box<dyn Error>> {
    let mut buffer: Vec<u8> = vec![0u8; CHUNK_SIZE];
    let len = self.file.read(&mut buffer).expect("Cannot read file");
    if len == 0 {
      Err("EOF")?
    } else {
      self.buffer = buffer;
      self.offset += len;
      Ok(())
    }
  }
}

fn parse_mp3_frame(input: &[u8]) -> IResult<&[u8], Vec<u8>> {
  let (input, result) = bytes::streaming::take_till::<_, _, error::Error<_>>(|b| b == 0xff)(input)?;
  let (_input, word) = number::streaming::be_u16(input)?;
  if (word & 0xffe0) != 0xffe0 {
    return Err(nom::Err::Error(error::Error::new(input, error::ErrorKind::Tag)));
  }
  Ok((input, result.to_vec()))
}

impl Iterator for FrameParser {
  type Item = Vec<u8>;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      match parse_mp3_frame(&self.buffer) {
        Ok((remaining, frame)) => {
          self.buffer = remaining.to_vec();
          return Some(frame);
        }
        Err(Incomplete(_)) => {
          println!("Need more data");
          match self.read_more() {
            Ok(_) => continue,
            Err(_) => return None, // EOF
          }
        }
        Err(e) => {
          panic!("Parse error: {}", e);
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {

  #[test]
  fn test_iterator() {
    let mut file_iter = crate::FrameParser::new("samples/4tink.mp3").unwrap();
    let buffer = file_iter.next().unwrap();
    println!("Parsed line: {}", String::from_utf8_lossy(&*buffer));
  }
}
