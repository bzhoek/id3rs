use std::convert::TryInto;
use std::io::{Read, Seek};
use std::io::SeekFrom::Current;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// https://id3.org/id3v2.3.0
// https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-structure.html

fn main() -> Result<()> {
  let mut file = std::fs::File::open("/Users/bas/OneDrive/PioneerDJ/melodic/39. Deep in the Dark (feat. LENN V) [Fur Coat Remix] -- D-Nox [1279108732].mp3")?;
  let mut buffer = [0; 10];
  file.read_exact(&mut buffer)?;
  let signature = String::from_utf8(buffer[0..3].to_owned()).unwrap();
  let max = syncsafe(&buffer[6..10]);
  let mut pos = 0u64;
  println!("sig {} size {} {:?}", signature, max, buffer);
  loop {
    file.read_exact(&mut buffer)?;
    let idval = u32::from_be_bytes(buffer[0..4].try_into().unwrap());
    if idval == 0 { break; }
    let idstr = String::from_utf8(buffer[0..4].to_owned()).unwrap();
    let size = syncsafe(&buffer[4..8]);
    pos = file.seek(Current(size as i64)).unwrap();
    println!("frame {} size {} pos {} {:?}", idstr, size, pos, buffer);
  }
  Ok(())
}

// only 7 bytes of each byte are significant
fn syncsafe(bytes: &[u8]) -> u64 {
  bytes.iter().fold(0, |result, byte| { result << 7 | (*byte as u64) })
}

#[cfg(test)]
mod tests {
  use crate::syncsafe;

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
}