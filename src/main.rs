use std::convert::TryInto;
use std::io::{Read, Seek};
use std::io::SeekFrom::Current;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-structure.html

fn main() -> Result<()> {
  let mut file = std::fs::File::open("/Users/bas/OneDrive/PioneerDJ/melodic/39. Deep in the Dark (feat. LENN V) [Fur Coat Remix] -- D-Nox [1279108732].mp3")?;
  let mut buffer = [0; 10];
  file.read_exact(&mut buffer)?;
  let signature = String::from_utf8(buffer[0..3].to_owned()).unwrap();
  let size = u32::from_be_bytes(buffer[4..8].try_into().unwrap());
  println!("sig {} {:?}", signature, buffer);
  while true {
    file.read_exact(&mut buffer)?;
    let id = String::from_utf8(buffer[0..4].to_owned()).unwrap();
    let size = u32::from_be_bytes(buffer[4..8].try_into().unwrap());
    println!("frame {} size {} {:?}", id, size, buffer);
    file.seek(Current(size as i64)).unwrap();
  }
  Ok(())
}
