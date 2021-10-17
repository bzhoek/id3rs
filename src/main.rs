use std::convert::TryInto;
use std::fs::File;
use std::io::{Read, Seek};
use std::io::SeekFrom::Current;

use env_logger::Env;
use env_logger::Target::Stdout;
use log::debug;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub fn configure_logging() {
  env_logger::Builder::from_env(Env::default().default_filter_or("debug")).target(Stdout).init();
  debug!("Debug logging");
}

// https://id3.org/id3v2.3.0
// https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-structure.html
// https://www.the-roberts-family.net/metadata/mp3.html

pub struct Tags {
  #[allow(dead_code)]
  source: File,
  buffer: [u8; 10],
}

impl Tags {
  pub fn read_from(path: &str) -> Result<Tags> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = [0; 10];
    file.read_exact(&mut buffer)?;
    Ok(Tags { source: file, buffer: buffer })
  }

  pub fn version(&self) -> u8 {
    self.buffer[3]
  }

  pub fn extended(&self) -> bool {
    self.buffer[4] & 0b01000000 > 0
  }

  pub fn flags(&self) -> u8 {
    self.buffer[5]
  }
}

fn main() -> Result<()> {
  configure_logging();
  let filepath = std::env::args().nth(1).expect("No filepath given.");
  let mut file = std::fs::File::open(filepath)?;

  let mut header = [0; 10];
  file.read_exact(&mut header)?;
  let signature = String::from_utf8(header[0..3].to_owned()).unwrap();
  let version = header[3];
  let flags = header[5];
  let max = syncsafe(&header[6..10]);
  debug!("sig {} version {} flags {} size {} {:?}", signature, version, flags, max, header);
  loop {
    let pos = file.seek(Current(0)).unwrap();
    file.read_exact(&mut header)?;
    let idval = u32::from_be_bytes(header[0..4].try_into().unwrap());
    if idval == 0 { break; }

    let idstr = String::from_utf8(header[0..4].to_owned()).unwrap();
    let size = syncsafe(&header[4..8]);
    debug!("frame {} at {} size {} {:?}", idstr, pos, size, header);

    let mut bytes: Vec<u8> = vec![0; size as usize];
    file.read_exact(&mut *bytes)?;

    if idstr == "GEOB" {
      // https://stackoverflow.com/a/42067321/10326604
      let mut pos = 1;
      let mimetype = string_from_bytes(&bytes[pos..].to_vec());
      debug!("{} {:?}", pos, mimetype);
      pos += mimetype.1;
      let filename = string_from_bytes(&bytes[pos..].to_vec());
      debug!("{} {:?}", pos, filename);
      pos += filename.1;
      let description = string_from_bytes(&bytes[pos..].to_vec());
      debug!("{:?}", description);
      pos += description.1;
      let content = string_from_bytes(&bytes[pos..].to_vec());
      debug!("{:?}", content);
    } else if idstr == "APIC" {
      let mut pos = 1;
      let mimetype = string_from_bytes(&bytes[pos..].to_vec());
      debug!("{} {:?}", pos, mimetype);
      pos += mimetype.1 + 1;
      let description = string_from_bytes(&bytes[pos..].to_vec());
      debug!("{:?}", description);
      let size = bytes.len() - (pos + description.1);
      debug!("{:?}", size);
    } else if idstr == "COMM" {
      let mut pos = 4;
      let description = string_from_bytes(&bytes[pos..].to_vec());
      debug!("{:?}", description);
      pos += description.1;
      let content = string_from_bytes(&bytes[pos..].to_vec());
      debug!("{:?}", content);
    } else if idstr == "TXXX" {
      let mut pos = 1;
      let description = string_from_bytes(&bytes[pos..].to_vec());
      debug!("{:?}", description);
      pos += description.1;
      let content = string_from_bytes(&bytes[pos..].to_vec());
      debug!("{:?}", content);
    } else if idstr.starts_with("T") {
      let text = string_from_bytes(&bytes[1..].to_vec());
      debug!("{:?}", text);
    }
  }
  Ok(())
}

// https://id3.org/id3v2.4.0-structure
// Frames that allow different types of text encoding contains a text
// encoding description byte. Possible encodings:
//
// $00   ISO-8859-1 [ISO-8859-1]. Terminated with $00.
// $01   UTF-16 [UTF-16] encoded Unicode [UNICODE] with BOM. All
// strings in the same frame SHALL have the same byteorder.
// Terminated with $00 00.
// $02   UTF-16BE [UTF-16] encoded Unicode [UNICODE] without BOM.
// Terminated with $00 00.
// $03   UTF-8 [UTF-8] encoded Unicode [UNICODE]. Terminated with $00.
fn string_from_bytes(bytes: &Vec<u8>) -> (String, usize) {
  // https://stackoverflow.com/q/36251992/10326604
  if bytes[0] == 0xff && bytes[1] == 0xfe {
    let words: Vec<u16> = bytes[2..]
      .chunks_exact(2)
      .into_iter()
      .map(|a| u16::from_ne_bytes([a[0], a[1]]))
      .collect();
    let len = words.iter()
      .position(|&c| c == 0)
      .unwrap_or(words.len());
    (String::from_utf16(&words[..len]).unwrap(), 2 + len * 2 + 2)
  } else {
    let len = bytes.iter()
      .position(|&c| c == b'\0')
      .unwrap_or(bytes.len());
    (String::from_utf8(bytes[..len].to_owned()).unwrap(), 1 + len)
  }
}

// only 7 bits of each byte are significant
fn syncsafe(bytes: &[u8]) -> u64 {
  bytes.iter().fold(0, |result, byte| { result << 7 | (*byte as u64) })
}

#[cfg(test)]
mod tests {
  use crate::{string_from_bytes, syncsafe, Tags};

  #[test]
  fn test_frame() {
    let bytes = [0u8, 0, 0x73, 0x71];
    let result = syncsafe(&bytes);
    assert_eq!(14833, result);
  }

  #[test]
  fn test_header() {
    let bytes = [0u8, 0x02, 0x3e, 0x77];
    let result = syncsafe(&bytes);
    assert_eq!(40823, result);
  }

  #[test]
  fn find_u8_eol() {
    let bytes: Vec<u8> = vec!(b'j', b's', b'o', b'n', 0);
    let result = string_from_bytes(&bytes);
    assert_eq!(("json".to_owned(), 5), result);
  }

  #[test]
  fn find_u16_eol() {
    let words: [u16; 5] = [0xfeff, 0x0043, 0x0075, 0x0065, 0];
    let bytes: Vec<[u8; 2]> = words.iter().map(|w| [*w as u8, (w >> 8) as u8]).collect();
    let bytes: Vec<u8> = bytes.iter().flat_map(|b| b.to_vec()).collect();
    let result = string_from_bytes(&bytes);
    assert_eq!(("Cue".to_owned(), 10), result);
  }

  #[test]
  fn test_tags() {
    let tags = Tags::read_from("/Users/bas/OneDrive/PioneerDJ/melodic/39. Deep in the Dark (feat. LENN V) [Fur Coat Remix] -- D-Nox [1279108732].mp3").unwrap();
    assert_eq!(4, tags.version());
    assert!(!tags.extended())
  }
}