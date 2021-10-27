use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::process::Command;

use log::{debug, LevelFilter};

use crate::patterns::{all_frames_v23, all_frames_v24, as_syncsafe, file_header};

mod mp3;
mod patterns;

#[derive(Debug, PartialEq, Eq)]
pub struct Header {
  version: u8,
  revision: u8,
  flags: u8,
  tag_size: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Frames {
  Frame {
    id: String,
    size: u32,
    flags: u16,
    data: Vec<u8>,
  },
  ExtendedText {
    id: String,
    size: u32,
    flags: u16,
    description: String,
    value: String,
  },
  Text {
    id: String,
    size: u32,
    flags: u16,
    text: String,
  },
  Object {
    id: String,
    size: u32,
    flags: u16,
    mime_type: String,
    filename: String,
    description: String,
    data: Vec<u8>,
  },
  Padding {
    size: u32
  },
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct ID3Tag {
  pub filepath: String,
  pub frames: Vec<Frames>,
}

const ID3HEADER_SIZE: u64 = 10;

impl ID3Tag {
  pub fn read(filepath: &str) -> Result<ID3Tag> {
    let (mut file, header) = Self::read_header(filepath)?;
    let mut input = vec![0u8; header.tag_size as usize];
    file.read_exact(&mut input).unwrap();

    let (_, result) = match header.version {
      3 => all_frames_v23(&input).map_err(|_| "Frames error")?,
      4 => all_frames_v24(&input).map_err(|_| "Frames error")?,
      v => Err(format!("Invalid version: {}", v))?
    };

    Ok(ID3Tag { filepath: filepath.to_string(), frames: result })
  }

  fn read_header(filepath: &str) -> Result<(File, Header)> {
    let mut file = std::fs::File::open(filepath)?;
    let mut buffer = [0; 10];
    file.read_exact(&mut buffer).unwrap();
    let (_, header) = file_header(&buffer).map_err(|_| "Header error")?;
    Ok((file, header))
  }

  pub fn write(&self, target: &str) -> Result<()> {
    let (mut file, header) = Self::read_header(&*self.filepath)?;

    let mut out = if self.filepath == target {
      let mut tmp = File::create("stream.tmp")?;
      file.seek(SeekFrom::Start(ID3HEADER_SIZE + header.tag_size as u64))?; // skip header and tag
      std::io::copy(&mut file, &mut tmp)?;
      OpenOptions::new().write(true).truncate(true).open(&self.filepath)?
    } else {
      File::create(target)?
    };

    out.write(b"ID3\x04\x00\x00FAKE")?;

    ID3Tag::write_id3_frames(&self.frames, &mut out)?;

    let size = out.stream_position()? - ID3HEADER_SIZE;
    debug!("new tag size {}", size);
    let vec = as_syncsafe(size as u32);
    out.seek(SeekFrom::Start(6))?;
    out.write(&*vec)?;
    out.seek(SeekFrom::Start(ID3HEADER_SIZE + size))?;

    if self.filepath == target {
      let mut tmp = File::open("stream.tmp")?;
      std::io::copy(&mut tmp, &mut out)?;
    } else {
      file.seek(SeekFrom::Start(10 + header.tag_size as u64))?;
      std::io::copy(&mut file, &mut out)?;
    };

    Ok(())
  }

  fn write_id3_frames(frames: &Vec<Frames>, out: &mut File) -> Result<()> {
    for frame in frames.iter() {
      match frame {
        Frames::Frame { id, size, flags, data } => {
          out.write(id.as_ref())?;
          let vec = as_syncsafe(*size);
          out.write(&*vec)?;
          out.write(&flags.to_be_bytes())?;
          out.write(&data)?;
        }
        Frames::Text { id, size: _, flags, text } => {
          let text: Vec<u8> = text.encode_utf16().map(|w| w.to_le_bytes()).flatten().collect();
          let len = text.len() as u32 + 3;
          let vec = as_syncsafe(len);
          out.write(b"T")?;
          out.write(id.as_ref())?;
          out.write(&*vec)?;
          out.write(&flags.to_be_bytes())?;
          out.write(b"\x01\xff\xfe")?;
          out.write(&*text)?;
        }
        Frames::ExtendedText { id, size: _, flags, description, value } => {
          let len = description.len() + value.len() + 2;
          let vec = as_syncsafe(len as u32);
          out.write(id.as_ref())?;
          out.write(&*vec)?;
          out.write(&flags.to_be_bytes())?;
          out.write(b"\x03")?;
          out.write(description.as_bytes())?;
          out.write(b"\x00")?;
          out.write(value.as_bytes())?;
        }
        _ => {}
      }
    }
    Ok(())
  }

  pub fn text(&self, identifier: &str) -> Option<String> {
    self.frames.iter().find(|f| match f {
      Frames::Text { id, size: _, flags: _, text: _ } => (id == identifier),
      _ => false
    }).map(|f| match f {
      Frames::Text { id: _, size: _, flags: _, text } => Some(text.to_string()),
      _ => None
    }).flatten()
  }

  pub fn objects(&self, identifier: &str) -> Vec<&Frames> {
    self.frames.iter().filter(|f| match f {
      Frames::Object { id, .. } => (id == identifier),
      _ => false
    }).collect()
  }

  pub fn object_by_filename(&self, name: &str) -> Option<&Frames> {
    self.frames.iter().find(|f| match f {
      Frames::Object { id, filename, .. } => (id == "GEOB" && filename == name),
      _ => false
    })
  }

  pub fn extended_text_frame(&self, name: &str) -> Option<&Frames> {
    self.frames.iter().find(|f| match f {
      Frames::ExtendedText { description, .. } => (description == name),
      _ => false
    })
  }

  pub fn extended_text(&self, name: &str) -> Option<&String> {
    self.extended_text_frame(name).map(|f| match f {
      Frames::ExtendedText { value, .. } => Some(value),
      _ => None
    }).flatten()
  }

  pub fn title(&self) -> Option<String> {
    self.text("IT2")
  }

  pub fn subtitle(&self) -> Option<String> {
    self.text("IT3")
  }

  pub fn artist(&self) -> Option<String> {
    self.text("PE1")
  }

  pub fn genre(&self) -> Option<String> {
    self.text("CON")
  }

  pub fn key(&self) -> Option<String> {
    self.text("KEY")
  }

  pub fn set_title(&mut self, text: &str) {
    self.set_text("IT2", text);
  }

  pub fn set_genre(&mut self, text: &str) {
    self.set_text("CON", text);
  }

  fn set_text(&mut self, id3: &str, change: &str) {
    if let Some(index) = self.frames.iter().position(|frame|
      match frame {
        Frames::Text { id, size: _, flags: _, text: _ } => id == id3,
        _ => false
      }) {
      self.frames.remove(index);
    }
    self.frames.push(Frames::Text { id: id3.to_string(), size: 0, flags: 0, text: change.to_string() })
  }

  pub fn set_extended_text(&mut self, name: &str, value: &str) {
    if let Some(index) = self.frames.iter().position(|frame|
      match frame {
        Frames::ExtendedText { description, .. } => description == name,
        _ => false
      }) {
      self.frames.remove(index);
    }
    self.frames.push(Frames::ExtendedText { id: "TXXX".to_string(), size: 0, flags: 0, description: name.to_string(), value: value.to_string() })
  }
}

pub fn make_rwcopy(rofile: &str, rwfile: &str) -> Result<()> {
  fs::copy(&rofile, &rwfile)?;
  let mut perms = fs::metadata(&rwfile)?.permissions();
  perms.set_readonly(false);
  fs::set_permissions(&rwfile, perms)?;
  Ok(())
}

pub fn mpck(filepath: &str) -> String {
  let output = Command::new("mpck")
    .arg(filepath)
    .output()
    .expect("failed to execute process");

  String::from_utf8(output.stdout).unwrap().replace(filepath, "")
}

pub fn log_init() {
  let _ = env_logger::builder().is_test(true)
    .filter_level(LevelFilter::Debug)
    .try_init();
}

#[cfg(test)]
mod tests {
  use std::convert::TryInto;

  use super::*;

  mod v23 {
    use super::*;

    #[test]
    pub fn test_geobs() {
      log_init();
      let (rofile, _, _) = filenames("3eep");
      let tag = ID3Tag::read(&rofile).unwrap();
      let data = "Hello, world".as_bytes().to_vec();
      assert_eq!(tag.objects("GEOB"), vec![&Frames::Object {
        id: "GEOB".to_string(),
        size: 121,
        flags: 0,
        mime_type: "application/vnd.rekordbox.dat".to_string(),
        filename: "ANLZ0000.DAT".to_string(),
        description: "Rekordbox Analysis Data".to_string(),
        data,
      }]);
    }

    #[test]
    pub fn test_geob_filename() {
      log_init();
      let (rofile, _, _) = filenames("3eep");
      let tag = ID3Tag::read(&rofile).unwrap();
      let data = "Hello, world".as_bytes().to_vec();
      let option = tag.object_by_filename("ANLZ0000.DAT");
      assert_eq!(option, Some(&Frames::Object {
        id: "GEOB".to_string(),
        size: 121,
        flags: 0,
        mime_type: "application/vnd.rekordbox.dat".to_string(),
        filename: "ANLZ0000.DAT".to_string(),
        description: "Rekordbox Analysis Data".to_string(),
        data,
      }));
    }

    #[test]
    pub fn test_extended_text_8859() {
      log_init();
      let (rofile, _, _) = filenames("3eep");
      let tag = ID3Tag::read(&rofile).unwrap();
      assert_eq!(tag.extended_text_frame("Hello"), Some(&Frames::ExtendedText {
        id: "TXXX".to_string(),
        size: 12,
        flags: 0,
        description: "Hello".to_string(),
        value: "World".to_string(),
      }));
    }

    #[test]
    pub fn test_extended_text_utf16() {
      log_init();
      let (rofile, _, _) = filenames("3eep-utf16");
      let tag = ID3Tag::read(&rofile).unwrap();
      assert_eq!(tag.extended_text_frame("Hello"), Some(&Frames::ExtendedText {
        id: "TXXX".to_string(),
        size: 23,
        flags: 0,
        description: "Hello".to_string(),
        value: "今日は".to_string(),
      }));
      assert_eq!(tag.extended_text("Hello"), Some(&"今日は".to_string()));
    }
  }

  #[test]
  pub fn test_invalid_version() {
    let (rofile, _, _) = filenames("5eep");
    let result = ID3Tag::read(&rofile).err().unwrap().to_string();
    assert_eq!(result, "Invalid version: 5".to_string());
  }

  mod v24 {
    use super::*;

    #[test]
    pub fn test_change_extended_text() {
      log_init();

      let (rofile, _, rwfile) = filenames("4eep");
      make_rwcopy(&rofile, &rwfile).unwrap();

      let mut tag = ID3Tag::read(&rwfile).unwrap();
      tag.set_extended_text("OriginalTitle", &tag.title().unwrap());
      tag.set_extended_text("EnergyLevel", "99");
      tag.write(&rwfile).unwrap();

      let tag = ID3Tag::read(&rwfile).unwrap();
      assert_eq!(tag.extended_text("OriginalTitle"), Some(&"Wild Eep".to_string()));
      assert_eq!(tag.extended_text("EnergyLevel"), Some(&"99".to_string()));
      assert_eq!(mpck(&rofile), mpck(&rwfile));
    }

    #[test]
    pub fn test_utf8_energy_level() {
      log_init();
      let (rofile, _, _) = filenames("4eep");
      let tag = ID3Tag::read(&rofile).unwrap();
      assert_eq!(tag.extended_text("Hello"), Some(&"World".to_string()));
    }

    #[test]
    pub fn test_reading() {
      log_init();
      let (rofile, _, _) = filenames("4bleak");
      let tag = ID3Tag::read(&rofile).unwrap();

      assert_eq!(tag.text("IT2"), Some("Bleak".to_string()));
      assert_eq!(tag.extended_text("EnergyLevel"), Some(&"6".to_string()));
      assert_eq!(tag.extended_text("OriginalTitle"), None);
      assert_eq!(tag.title(), Some("Bleak".to_string()));
      assert_eq!(tag.subtitle(), Some("".to_string()));
      assert_eq!(tag.key(), Some("4A".to_string()));
      assert_eq!(tag.artist(), Some("Maenad Veyl".to_string()));
    }
  }

  #[test]
  pub fn test_reading_genre() {
    log_init();
    let (rofile, _, _) = filenames("4blitz");
    let tag = ID3Tag::read(&rofile).unwrap();

    assert_eq!(tag.text("CON"), Some("techno".to_string()));
    assert_eq!(tag.genre(), Some("techno".to_string()));
  }

  #[test]
  pub fn test_changing_genre() {
    log_init();
    let (rofile, _, rwfile) = filenames("4blitz");
    make_rwcopy(&rofile, &rwfile).unwrap();

    let mut tag = ID3Tag::read(&rwfile).unwrap();
    assert_eq!(tag.text("CON"), Some("techno".to_string()));
    assert_eq!(tag.genre(), Some("techno".to_string()));
    tag.set_genre("notech");
    tag.write(&rwfile).unwrap();

    let tag = ID3Tag::read(&rwfile).unwrap();
    assert_eq!(tag.genre(), Some("notech".to_string()));
    assert_eq!(mpck(&rofile), mpck(&rwfile));
  }

  #[test]
  pub fn test_unmodified_frame_count() {
    log_init();
    let (rofile, _, _) = filenames("4bleak");

    let tag = ID3Tag::read(&rofile).unwrap();
    assert_eq!(tag.frames.len(), 14);
    assert_eq!(tag.extended_text("OriginalTitle"), None);
  }

  #[test]
  pub fn test_change_copy() {
    log_init();
    let (rofile, outfile, _) = filenames("4bleak");

    let mut tag = ID3Tag::read(&rofile).unwrap();
    tag.set_title("Bleek");
    tag.set_extended_text("EnergyLevel", "99");
    tag.write(&outfile).unwrap();
    assert_eq!(mpck(&rofile), mpck(&outfile));
  }

  #[test]
  pub fn test_change_inplace() {
    log_init();
    let (rofile, _, rwfile) = filenames("4bleak");
    make_rwcopy(&rofile, &rwfile).unwrap();

    let mut tag = ID3Tag::read(&rwfile).unwrap();
    tag.set_title("Bleek");
    tag.set_extended_text("EnergyLevel", "99");
    tag.write(&rwfile).unwrap();
    assert_eq!(mpck(&rofile), mpck(&rwfile));
  }

  #[test]
  pub fn test_sync_safe() {
    log_init();

    assert_eq!(as_syncsafe_bytes(66872), 0x040A38);
    assert_eq!(as_syncsafe_bytes(0b00001111111_1111111_1111111_1111111u32), 0b01111111011111110111111101111111u32);

    assert_eq!(as_syncsafe(0b1111111_1111111u32), vec![0, 0, 127, 127]);
    assert_eq!(as_syncsafe(0b1111111_1111111_1111111u32), vec![0, 127, 127, 127]);
    assert_eq!(as_syncsafe(0b00001111111_1111111_1111111_1111111u32), vec![127, 127, 127, 127]);
  }

  fn as_syncsafe_bytes(total: u32) -> u32 {
    let vec = as_syncsafe(total);
    let (bytes, _) = vec.as_slice().split_at(std::mem::size_of::<u32>());
    u32::from_be_bytes(bytes.try_into().unwrap())
  }

  #[test]
  pub fn test_sum_frames() {
    log_init();
    let (rofile, _, _) = filenames("4bleak");

    let tag = ID3Tag::read(&rofile).unwrap();
    let sum = tag.frames.iter()
      .fold(0u32, |sum, frame| sum + match frame {
        Frames::Frame { size, .. } => (10 + size),
        Frames::Text { size, .. } => (10 + size),
        Frames::ExtendedText { size, .. } => (10 + size),
        Frames::Object { size, .. } => (10 + size),
        Frames::Padding { size } => (0 + size),
      });

    assert_eq!(sum, 42316);

    let _sum = tag.frames.iter()
      .fold(0u32, |sum, frame| sum + match frame {
        Frames::Frame { size, .. } => (10 + size),
        Frames::Text { text, .. } => (10 + 1 + text.len() as u32),
        Frames::ExtendedText { size, .. } => (10 + size),
        Frames::Object { size, .. } => (10 + size),
        Frames::Padding { size } => (0 + size),
      });

    let _double_utf16 = 15 + 23 + 11 + 3 + 15 + (5 * 2); // 67
  }

  fn filenames(base: &str) -> (String, String, String) {
    (format!("{}.mp3", base), format!("{}-out.mp3", base), format!("{}-rw.mp3", base))
  }
}