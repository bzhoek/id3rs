use std::convert::TryInto;
use std::fs::File;
use std::io::{Read, Seek};
use std::io::SeekFrom::Current;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// https://id3.org/id3v2.3.0
// https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-structure.html
// https://www.the-roberts-family.net/metadata/mp3.html

pub struct Tags {
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
  let mut file = std::fs::File::open("/Users/bas/OneDrive/PioneerDJ/melodic/39. Deep in the Dark (feat. LENN V) [Fur Coat Remix] -- D-Nox [1279108732].mp3")?;
  let mut buffer = [0; 10];
  file.read_exact(&mut buffer)?;
  let signature = String::from_utf8(buffer[0..3].to_owned()).unwrap();
  let version = buffer[3];
  let flags = buffer[5];
  let max = syncsafe(&buffer[6..10]);
  println!("sig {} version {} flags {} size {} {:?}", signature, version, flags, max, buffer);
  loop {
    file.read_exact(&mut buffer)?;
    let idval = u32::from_be_bytes(buffer[0..4].try_into().unwrap());
    if idval == 0 { break; }
    let idstr = String::from_utf8(buffer[0..4].to_owned()).unwrap();
    let size = syncsafe(&buffer[4..8]);
    println!("frame {} size {} {:?}", idstr, size, buffer);

    if !idstr.starts_with("T") {
      file.seek(Current(size as i64)).unwrap();
      continue;
    }

    let mut text: Vec<u8> = vec![0; size as usize];
    file.read_exact(&mut *text)?;
    if text[0] == 1 {
      let title: Vec<u16> = text[3..]
        .chunks_exact(2)
        .into_iter()
        .map(|a| u16::from_ne_bytes([a[0], a[1]]))
        .collect();
      let title = title.as_slice();
      let title = String::from_utf16(title).unwrap();

      println!("text {:?}", title);
    } else {
      let title = String::from_utf8(text[1..].to_owned()).unwrap();
      println!("text {:?}", title);
    }
    // let pos = file.seek(Current(size as i64)).unwrap();
  }
  Ok(())
}

// only 7 bytes of each byte are significant
fn syncsafe(bytes: &[u8]) -> u64 {
  bytes.iter().fold(0, |result, byte| { result << 7 | (*byte as u64) })
}

#[cfg(test)]
mod tests {
  use walkdir::WalkDir;

  use crate::{syncsafe, Tags};

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
  fn test_tags() {
    let tags = Tags::read_from("/Users/bas/OneDrive/PioneerDJ/melodic/39. Deep in the Dark (feat. LENN V) [Fur Coat Remix] -- D-Nox [1279108732].mp3").unwrap();
    assert_eq!(4, tags.version());
    assert!(!tags.extended())
  }

  #[test]
  fn find_extended() {
    let walker = WalkDir::new("/Users/bas/OneDrive/PioneerDJ").into_iter();
    for entry in walker {
      let entry = entry.unwrap();
      let path = entry.path().to_str().unwrap();
      if path.ends_with(".mp3") {
        println!("{}", path);
        let tags = Tags::read_from(path).unwrap();
        assert_eq!(0, tags.flags(), "{:?}", entry);
      }
    }
  }
}