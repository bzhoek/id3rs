use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use log::{debug, LevelFilter};

use crate::parsers::{all_frames, as_syncsafe, file_header, v23_len, v24_len};

pub static TITLE_TAG: &str = "TIT2";
pub static SUBTITLE_TAG: &str = "TIT3";
pub static ARTIST_TAG: &str = "TPE1";
pub static GENRE_TAG: &str = "TCON";
pub static KEY_TAG: &str = "TKEY";
pub static COMMENT_TAG: &str = "COMM";
pub static OBJECT_TAG: &str = "GEOB";
pub static GROUPING_TAG: &str = "GRP1";
pub static EXTENDED_TAG: &str = "TXXX";
pub static PICTURE_TAG: &str = "APIC";

pub mod frame;
pub mod parsers;

#[derive(Debug, PartialEq, Eq)]
pub struct Header {
  pub version: u8,
  pub revision: u8,
  pub flags: u8,
  pub tag_size: u32,
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
      3 => all_frames(v23_len)(&input).map_err(|_| "Frames error")?,
      4 => all_frames(v24_len)(&input).map_err(|_| "Frames error")?,
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

pub fn log_init() {
  let _ = env_logger::builder().is_test(true)
    .filter_level(LevelFilter::Debug)
    .try_init();
}