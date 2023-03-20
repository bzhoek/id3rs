use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::Command;

use log::{debug, LevelFilter};

use crate::patterns::{all_frames_v23, all_frames_v24, as_syncsafe, file_header};

static TITLE_TAG: &str = "TIT2";
static SUBTITLE_TAG: &str = "TIT3";
static ARTIST_TAG: &str = "TPE1";
static GENRE_TAG: &str = "TCON";
static KEY_TAG: &str = "TKEY";
static COMMENT_TAG: &str = "COMM";
static OBJECT_TAG: &str = "GEOB";
static GROUPING_TAG: &str = "GRP1";
static EXTENDED_TAG: &str = "TXXX";
static PICTURE_TAG: &str = "APIC";

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
pub enum Frame {
  Generic {
    id: String,
    size: u32,
    flags: u16,
    data: Vec<u8>,
  },
  Comment {
    id: String,
    size: u32,
    flags: u16,
    language: String,
    description: String,
    value: String,
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
  Picture {
    id: String,
    size: u32,
    flags: u16,
    mime_type: String,
    kind: u8,
    description: String,
    data: Vec<u8>,
  },
  Padding {
    size: u32
  },
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct ID3rs {
  pub filepath: String,
  pub frames: Vec<Frame>,
  pub dirty: bool,
}

const ID3HEADER_SIZE: u64 = 10;

impl ID3rs {
  pub fn read(path: impl AsRef<Path> + Copy) -> Result<ID3rs> {
    let (mut file, header) = Self::read_header(path)?;
    let mut input = vec![0u8; header.tag_size as usize];
    file.read_exact(&mut input).unwrap();

    let (_, result) = match header.version {
      3 => all_frames_v23(&input).map_err(|_| "Frames error")?,
      4 => all_frames_v24(&input).map_err(|_| "Frames error")?,
      v => Err(format!("Invalid version: {}", v))?
    };

    Ok(ID3rs { filepath: path.as_ref().to_str().unwrap().to_string(), frames: result, dirty: false })
  }

  fn read_header(path: impl AsRef<Path>) -> Result<(File, Header)> {
    let mut file = File::open(path)?;
    let mut buffer = [0; 10];
    file.read_exact(&mut buffer).unwrap();
    let (_, header) = file_header(&buffer).map_err(|_| "Header error")?;
    Ok((file, header))
  }

  pub fn write(&self, target: impl AsRef<Path>) -> Result<()> {
    let (mut file, header) = Self::read_header(&*self.filepath)?;

    let mut tmp: File = tempfile::tempfile()?;

    let target = target.as_ref().to_str().unwrap().to_string();
    let mut out = if self.filepath == target {
      file.seek(SeekFrom::Start(ID3HEADER_SIZE + header.tag_size as u64))?; // skip header and tag
      std::io::copy(&mut file, &mut tmp)?;
      OpenOptions::new().write(true).truncate(true).open(&self.filepath)?
    } else {
      File::create(&target)?
    };

    out.write(b"ID3\x04\x00\x00FAKE")?;

    ID3rs::write_id3_frames(&self.frames, &mut out)?;

    let size = out.stream_position()? - ID3HEADER_SIZE;
    debug!("new tag size {}", size);
    let vec = as_syncsafe(size as u32);
    out.seek(SeekFrom::Start(6))?;
    out.write(&*vec)?;
    out.seek(SeekFrom::Start(ID3HEADER_SIZE + size))?;

    if self.filepath == target {
      tmp.seek(SeekFrom::Start(0))?;
      std::io::copy(&mut tmp, &mut out)?;
    } else {
      file.seek(SeekFrom::Start(10 + header.tag_size as u64))?;
      std::io::copy(&mut file, &mut out)?;
    };

    Ok(())
  }

  fn write_id3_frames(frames: &Vec<Frame>, out: &mut File) -> Result<()> {
    for frame in frames.iter() {
      match frame {
        Frame::Generic { id, size, flags, data } => {
          out.write(id.as_ref())?;
          let vec = as_syncsafe(*size);
          debug!("frame {} len {}", id, size);
          out.write(&*vec)?;
          out.write(&flags.to_be_bytes())?;
          out.write(&data)?;
        }
        Frame::Text { id, size: _, flags, text } => {
          let text: Vec<u8> = text.encode_utf16().map(|w| w.to_le_bytes()).flatten().collect();
          let len = text.len() as u32 + 3;
          let size = as_syncsafe(len);
          debug!("text {} len {}", id, len);
          out.write(id.as_ref())?;
          out.write(&*size)?;
          out.write(&flags.to_be_bytes())?;

          out.write(b"\x01\xff\xfe")?;
          out.write(&*text)?;
        }
        Frame::Comment { id, size: _, flags, language, description, value } => {
          let len = language.len() + description.len() + value.len() + 2;
          let size = as_syncsafe(len as u32);
          debug!("comment {} len {}", id, len);
          out.write(id.as_ref())?;
          out.write(&*size)?;
          out.write(&flags.to_be_bytes())?;

          out.write(b"\x03")?;
          out.write(language.as_bytes())?;
          out.write(description.as_bytes())?;
          out.write(b"\x00")?;
          out.write(value.as_bytes())?;
        }
        Frame::ExtendedText { id, size: _, flags, description, value } => {
          let len = description.len() + value.len() + 2;
          let size = as_syncsafe(len as u32);
          debug!("extended {} len {}", id, len);
          out.write(id.as_ref())?;
          out.write(&*size)?;
          out.write(&flags.to_be_bytes())?;

          out.write(b"\x03")?;
          out.write(description.as_bytes())?;
          out.write(b"\x00")?;
          out.write(value.as_bytes())?;
        }
        Frame::Object { id, flags, mime_type, filename, description, data, .. } => {
          let len = mime_type.len() + filename.len() + description.len() + 4 + data.len();
          let size = as_syncsafe(len as u32);
          debug!("object {} len {}", id, len);
          out.write(id.as_ref())?;
          out.write(&*size)?;
          out.write(&flags.to_be_bytes())?;

          out.write(b"\x03")?;
          out.write(mime_type.as_bytes())?;
          out.write(b"\x00")?;
          out.write(filename.as_bytes())?;
          out.write(b"\x00")?;
          out.write(description.as_bytes())?;
          out.write(b"\x00")?;
          out.write(data)?;
        }
        Frame::Picture { id, flags, kind, mime_type, description, data, .. } => {
          let len = mime_type.len() + description.len() + 4 + data.len();
          let size = as_syncsafe(len as u32);
          debug!("picture {} len {}", id, len);
          out.write(id.as_ref())?;
          out.write(&*size)?;
          out.write(&flags.to_be_bytes())?;

          out.write(b"\x03")?;
          out.write(mime_type.as_bytes())?;
          out.write(b"\x00")?;
          out.write(&kind.to_be_bytes())?;
          out.write(description.as_bytes())?;
          out.write(b"\x00")?;
          out.write(data)?;
        }
        _ => {}
      }
    }
    Ok(())
  }

  pub fn text(&self, identifier: &str) -> Option<&str> {
    self.frames.iter().find(|f| match f {
      Frame::Text { id, .. } => id == identifier,
      _ => false
    }).map(|f| match f {
      Frame::Text { text, .. } => Some(text.as_str()),
      _ => None
    }).flatten()
  }

  pub fn comment(&self) -> Option<&str> {
    self.frames.iter().find(|f| match f {
      Frame::Comment { id, .. } => id == COMMENT_TAG,
      _ => false
    }).map(|f| match f {
      Frame::Comment { value, .. } => Some(value.as_str()),
      _ => None
    }).flatten()
  }

  pub fn objects(&self, identifier: &str) -> Vec<&Frame> {
    self.frames.iter().filter(|f| match f {
      Frame::Object { id, .. } => id == identifier,
      _ => false
    }).collect()
  }

  pub fn object_by_filename(&self, name: &str) -> Option<&Frame> {
    self.frames.iter().find(|f| match f {
      Frame::Object { id, filename, .. } => id == OBJECT_TAG && filename == name,
      _ => false
    })
  }

  pub fn extended_text(&self, name: &str) -> Option<&str> {
    self.extended_text_frame(name).map(|f| match f {
      Frame::ExtendedText { value, .. } => Some(value.as_str()),
      _ => None
    }).flatten()
  }

  pub fn extended_text_frame(&self, name: &str) -> Option<&Frame> {
    self.frames.iter().find(|f| match f {
      Frame::ExtendedText { description, .. } => description == name,
      _ => false
    })
  }

  pub fn attached_picture(&self, kind: u8) -> Option<&Frame> {
    self.frames.iter().find(|f| match f {
      Frame::Picture { kind: kind_, .. } => &kind == kind_,
      _ => false
    })
  }

  pub fn title(&self) -> Option<&str> {
    self.text(TITLE_TAG)
  }

  pub fn subtitle(&self) -> Option<&str> {
    self.text(SUBTITLE_TAG)
  }

  pub fn artist(&self) -> Option<&str> {
    self.text(ARTIST_TAG)
  }

  pub fn genre(&self) -> Option<&str> {
    self.text(GENRE_TAG)
  }

  pub fn key(&self) -> Option<&str> { self.text(KEY_TAG) }

  pub fn grouping(&self) -> Option<&str> { self.text(GROUPING_TAG) }

  pub fn set_title(&mut self, text: &str) {
    self.set_text(TITLE_TAG, text);
  }

  pub fn set_key(&mut self, text: &str) { self.set_text(KEY_TAG, text); }

  pub fn set_genre(&mut self, text: &str) {
    self.set_text(GENRE_TAG, text);
  }

  pub fn set_object(&mut self, name: &str, mime_type: &str, description: &str, data: &[u8]) {
    if let Some(index) = self.frames.iter().position(|frame|
      match frame {
        Frame::Object { id, filename, .. } => id == OBJECT_TAG && filename == name,
        _ => false
      }) {
      self.frames.remove(index);
    }
    self.push_new_frame(Frame::Object {
      id: OBJECT_TAG.to_string(),
      size: 0,
      flags: 0,
      filename: name.to_string(),
      description: description.to_string(),
      mime_type: mime_type.to_string(),
      data: Vec::from(data),
    })
  }

  fn set_text(&mut self, id3: &str, change: &str) {
    if let Some(index) = self.frames.iter().position(|frame|
      match frame {
        Frame::Text { id, .. } => id == id3,
        _ => false
      }) {
      self.frames.remove(index);
    }
    self.push_new_frame(Frame::Text { id: id3.to_string(), size: 0, flags: 0, text: change.to_string() });
  }

  fn push_new_frame(&mut self, frames: Frame) {
    self.frames.push(frames);
    self.dirty = true
  }

  pub fn set_comment(&mut self, description: &str, value: &str) {
    if let Some(index) = self.frames.iter().position(|frame|
      match frame {
        Frame::Comment { .. } => true,
        _ => false
      }) {
      self.frames.remove(index);
    }
    self.push_new_frame(Frame::Comment {
      id: COMMENT_TAG.to_string(),
      size: 0,
      flags: 0,
      language: "eng".to_string(),
      description: description.to_string(),
      value: value.to_string(),
    })
  }

  pub fn set_extended_text(&mut self, name: &str, value: &str) {
    if let Some(index) = self.frames.iter().position(|frame|
      match frame {
        Frame::ExtendedText { description, .. } => description == name,
        _ => false
      }) {
      self.frames.remove(index);
    }
    self.push_new_frame(Frame::ExtendedText { id: EXTENDED_TAG.to_string(), size: 0, flags: 0, description: name.to_string(), value: value.to_string() });
  }

  pub fn set_attached_picture(&mut self, kind: u8, mime_type: &str, description: &str, data: &[u8]) {
    if let Some(index) = self.frames.iter().position(|frame|
      match frame {
        Frame::Picture { kind: kind_, .. } => kind_ == &kind,
        _ => false
      }) {
      self.frames.remove(index);
    }
    self.push_new_frame(Frame::Picture { id: PICTURE_TAG.to_string(), size: 0, flags: 0, kind, mime_type: mime_type.to_string(), description: description.to_string(), data: Vec::from(data) });
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

    const FILENAME: &str = "samples/3tink";

    #[test]
    pub fn test_reading() {
      log_init();
      let (rofile, _, _) = filenames(FILENAME);
      let tag = ID3rs::read(&rofile).unwrap();

      assert_eq!(tag.text(TITLE_TAG), Some("Tink"));
      assert_eq!(tag.title(), Some("Tink"));
      assert_eq!(tag.artist(), Some("Apple"));
      assert_eq!(tag.comment(), Some("From Big Sur"));
    }

    #[test]
    pub fn test_all_geobs() {
      log_init();
      let (rofile, _, _) = filenames(FILENAME);
      let tag = ID3rs::read(&rofile).unwrap();
      let data = "Hello, world".as_bytes().to_vec();
      assert_eq!(tag.objects(OBJECT_TAG), vec![&Frame::Object {
        id: OBJECT_TAG.to_string(),
        size: 80,
        flags: 0,
        mime_type: "application/vnd.rekordbox.dat".to_string(),
        filename: "ANLZ0000.DAT".to_string(),
        description: "Rekordbox Analysis Data".to_string(),
        data,
      }]);
    }

    #[test]
    pub fn test_find_geob() {
      log_init();
      let (rofile, _, _) = filenames(FILENAME);
      let tag = ID3rs::read(&rofile).unwrap();
      let data = "Hello, world".as_bytes().to_vec();
      let option = tag.object_by_filename("ANLZ0000.DAT");
      assert_eq!(option, Some(&Frame::Object {
        id: OBJECT_TAG.to_string(),
        size: 80,
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
      let (rofile, _, _) = filenames(FILENAME);
      let tag = ID3rs::read(&rofile).unwrap();
      assert_eq!(tag.extended_text_frame("Hello"), Some(&Frame::ExtendedText {
        id: EXTENDED_TAG.to_string(),
        size: 12,
        flags: 0,
        description: "Hello".to_string(),
        value: "World".to_string(),
      }));
    }

    #[test]
    pub fn test_extended_text_utf16() {
      log_init();
      let (rofile, _, _) = filenames(FILENAME);
      let tag = ID3rs::read(&rofile).unwrap();
      assert_eq!(tag.extended_text_frame("こんにちは"), Some(&Frame::ExtendedText {
        id: EXTENDED_TAG.to_string(),
        size: 21,
        flags: 0,
        description: "こんにちは".to_string(),
        value: "世界".to_string(),
      }));
      assert_eq!(tag.extended_text("こんにちは"), Some("世界"));
    }
  }

  #[test]
  pub fn test_invalid_version() {
    let (rofile, _, _) = filenames("samples/5eep");
    let result = ID3rs::read(&rofile).err().unwrap().to_string();
    assert_eq!(result, "Invalid version: 5".to_string());
  }

  fn rw_test(stem: &str, body: fn(&(String, String, String))) {
    log_init();
    let names = filenames(stem);
    make_rwcopy(&names.0, &names.2).unwrap();
    body(&names);
    fs::remove_file(names.1).unwrap_or(());
    fs::remove_file(names.2).unwrap_or(())
  }

  mod v24 {
    use super::*;

    const FILENAME: &str = "samples/4tink";

    #[test]
    pub fn test_set_object() {
      rw_test(FILENAME, |(_, _, rwfile)| {
        let mut tag = ID3rs::read(&rwfile).unwrap();

        tag.set_object("HELLO.TXT", "text/plain", "Hello", &"Hello, world".as_bytes());
        tag.set_extended_text("EnergyLevel", "99");
        tag.write(&rwfile).unwrap();
      });
    }

    #[test]
    pub fn test_change_extended_text() {
      rw_test(FILENAME, |(rofile, _, rwfile)| {
        let mut tag = ID3rs::read(&rwfile).unwrap();
        tag.set_extended_text("OriginalTitle", &tag.title().unwrap().to_string());
        tag.set_extended_text("EnergyLevel", "99");
        tag.write(&rwfile).unwrap();

        let tag = ID3rs::read(&rwfile).unwrap();
        assert_eq!(tag.extended_text("OriginalTitle"), Some("Tink"));
        assert_eq!(tag.extended_text("EnergyLevel"), Some("99"));
        assert_eq!(mpck(&rofile), mpck(&rwfile));
      });
    }

    #[test]
    pub fn test_attach_picture() {
      rw_test(FILENAME, |(rofile, _, rwfile)| {
        let mut tag = ID3rs::read(&rwfile).unwrap();
        let data = fs::read("cover.jpg").unwrap();
        tag.set_attached_picture(03, "image/png", "cover", &*data);
        tag.write(&rwfile).unwrap();

        let tag = ID3rs::read(&rwfile).unwrap();
        let picture = tag.attached_picture(3);
        assert!(matches!(picture, Some(Frame::Picture { data, .. })));
      });
    }

    #[test]
    pub fn test_utf8_energy_level() {
      log_init();
      let (rofile, _, _) = filenames(FILENAME);
      let tag = ID3rs::read(&rofile).unwrap();
      assert_eq!(tag.extended_text("Hello"), Some("World"));
    }

    #[test]
    pub fn test_reading() {
      log_init();
      let (rofile, _, _) = filenames("samples/4tink");
      let tag = ID3rs::read(&rofile).unwrap();

      assert_eq!(tag.text(TITLE_TAG), Some("Tink"));
      assert_eq!(tag.extended_text("EnergyLevel"), Some("6"));
      assert_eq!(tag.extended_text("OriginalTitle"), None);
      assert_eq!(tag.title(), Some("Tink"));
      assert_eq!(tag.subtitle(), Some(""));
      assert_eq!(tag.key(), Some("4A"));
      assert_eq!(tag.grouping(), Some("2241"));
      assert_eq!(tag.artist(), Some("Apple"));
      assert_eq!(tag.comment(), Some("From Big Sur"));
    }
  }

  #[test]
  pub fn test_reading_genre() {
    log_init();
    let (rofile, _, _) = filenames("samples/4tink");
    let tag = ID3rs::read(&rofile).unwrap();

    assert_eq!(tag.text(GENRE_TAG), Some("sounds"));
    assert_eq!(tag.genre(), Some("sounds"));
  }

  #[test]
  pub fn test_changing_genre() {
    rw_test("samples/4tink", |(rofile, _, rwfile)| {
      let mut tag = ID3rs::read(&rwfile).unwrap();
      assert_eq!(tag.text(GENRE_TAG), Some("sounds"));
      assert_eq!(tag.genre(), Some("sounds"));
      tag.set_genre("notech");
      tag.write(&rwfile).unwrap();

      let tag = ID3rs::read(&rwfile).unwrap();
      assert_eq!(tag.genre(), Some("notech"));
      assert_eq!(mpck(&rofile), mpck(&rwfile));
    });
  }

  #[test]
  pub fn test_unmodified_frame_count() {
    log_init();
    let (rofile, _, _) = filenames("samples/4tink");

    let tag = ID3rs::read(&rofile).unwrap();
    assert_eq!(tag.frames.len(), 12);
    assert_eq!(tag.extended_text("OriginalTitle"), None);
  }

  #[test]
  pub fn test_change_comment() {
    rw_test("samples/4tink", |(rofile, outfile, _)| {
      let mut tag = ID3rs::read(&rofile).unwrap();
      tag.set_comment("", "New comment");
      tag.write(&outfile).unwrap();
      assert_eq!(mpck(&rofile), mpck(&outfile));
    });
  }

  #[test]
  pub fn test_change_copy() {
    rw_test("samples/4tink", |(rofile, outfile, _)| {
      let mut tag = ID3rs::read(&rofile).unwrap();
      tag.set_title("Bleek");
      tag.set_extended_text("EnergyLevel", "99");
      tag.write(&outfile).unwrap();
      assert_eq!(mpck(&rofile), mpck(&outfile));
    });
  }

  #[test]
  pub fn test_change_inplace() {
    rw_test("samples/4tink", |(rofile, _, rwfile)| {
      let mut tag = ID3rs::read(&rwfile).unwrap();
      tag.set_title("Bleek");
      tag.set_extended_text("EnergyLevel", "99");
      tag.write(&rwfile).unwrap();
      assert_eq!(mpck(&rofile), mpck(&rwfile));
    });
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
    let (rofile, _, _) = filenames("samples/4tink");

    let tag = ID3rs::read(&rofile).unwrap();
    let sum = tag.frames.iter()
      .fold(0u32, |sum, frame| sum + match frame {
        Frame::Generic { size, .. } => 10 + size,
        Frame::Text { size, .. } => 10 + size,
        Frame::Comment { size, .. } => 10 + size,
        Frame::ExtendedText { size, .. } => 10 + size,
        Frame::Object { size, .. } => 10 + size,
        Frame::Padding { size } => 0 + size,
        Frame::Picture { size, .. } => 10 + size,
      });

    assert_eq!(sum, 1114);

    let _sum = tag.frames.iter()
      .fold(0u32, |sum, frame| sum + match frame {
        Frame::Generic { size, .. } => 10 + size,
        Frame::Text { text, .. } => 10 + 1 + text.len() as u32,
        Frame::Comment { size, .. } => 10 + size,
        Frame::ExtendedText { size, .. } => 10 + size,
        Frame::Object { size, .. } => 10 + size,
        Frame::Padding { size } => 0 + size,
        Frame::Picture { size, .. } => 10 + size,
      });

    let _double_utf16 = 15 + 23 + 11 + 3 + 15 + (5 * 2); // 67
  }

  fn filenames(base: &str) -> (String, String, String) {
    let rnd = rand::random::<u32>();
    (format!("{}.mp3", base), format!("{}-out{}.mp3", base, rnd), format!("{}-rw{}.mp3", base, rnd))
  }
}