use std::convert::TryInto;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::str::from_utf8;

use log::{debug, error, info, LevelFilter, warn};
use nom::branch::alt;
use nom::bytes::streaming::{tag, take};
use nom::combinator::{eof, map};
use nom::IResult;
use nom::multi::{count, fold_many_m_n, many_till};
use nom::number::complete::be_u32;
use nom::number::streaming::{be_u16, be_u8, le_u16};
use nom::sequence::tuple;

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
  Text {
    id: String,
    size: u32,
    flags: u16,
    text: String,
  },
  Padding {
    size: u32
  },
}

fn file_header(input: &[u8]) -> IResult<&[u8], Header> {
  let (input, (_, version, revision, flags, tag_size))
    = tuple((tag("ID3"), be_u8, be_u8, be_u8, syncsafe))(input)?;
  debug!("ID3 {} tag size {}", version, tag_size);
  Ok((input, Header { version, revision, flags, tag_size }))
}

fn id_as_str(input: &[u8]) -> IResult<&[u8], &str> {
  map(
    take(4u8),
    |res| match from_utf8(res) {
      Ok(id) => id,
      Err(_) => "FAIL"
    },
  )(input)
}

fn syncsafe(input: &[u8]) -> IResult<&[u8], u32> {
  fold_many_m_n(4, 4, be_u8, 0u32,
    |acc, byte| acc << 7 | (byte as u32))(input)
}

fn generic_frame(input: &[u8]) -> IResult<&[u8], Frames> {
  let (input, (id, size, flags)) =
    tuple((id_as_str, syncsafe, be_u16))(input)?;
  debug!("frame {} {}", id, size);
  let (input, data) = take(size)(input)?;
  Ok((input, Frames::Frame { id: id.to_string(), size, flags, data: data.into() }))
}

fn text_header(input: &[u8]) -> IResult<&[u8], (&[u8], &str, u32, u16)> {
  tuple((
    tag("T"),
    map(
      take(3u8),
      |res| from_utf8(res).unwrap(),
    ),
    syncsafe,
    be_u16
  ))(input)
}

fn text_frame_utf16(input: &[u8]) -> IResult<&[u8], Frames> {
  let (input, (_, id, size, flags)) = text_header(input)?;
  debug!("utf16 {} {}", id, size);
  let (input, (_, text)) =
    tuple((
      tag(b"\x01\xff\xfe"),
      count(le_u16, (size - 3) as usize / 2)
    ))(input)?;
  let text: Vec<u16> = text.into_iter().filter(|w| *w != 0xfeffu16).collect();
  let text = String::from_utf16(&*text).unwrap().replace("\u{0000}", "\n");
  debug!("utf16 {}", text);
  Ok((input, Frames::Text { id: id.to_string(), size, flags, text }))
}

fn text_frame_utf8(input: &[u8]) -> IResult<&[u8], Frames> {
  let (input, (_, id, size, flags)) = text_header(input)?;
  let (input, (_, text)) =
    tuple((
      alt((tag(b"\x00"), tag(b"\x03"))),
      count(be_u8, (size - 1) as usize)
    ))(input)?;
  let text = String::from_utf8(text).unwrap().replace("\u{0}", "\n");
  debug!("utf8 {} {} {}", id, size, text);
  Ok((input, Frames::Text { id: id.to_string(), size, flags, text }))
}

fn text_frame(input: &[u8]) -> IResult<&[u8], Frames> {
  alt((text_frame_utf8, text_frame_utf16, ))(input)
}

fn padding(input: &[u8]) -> IResult<&[u8], Frames> {
  let (input, pad) =
    many_till(tag(b"\x00"), eof)
      (input)?;
  Ok((input, Frames::Padding { size: pad.0.len() as u32 }))
}

fn all_frames(input: &[u8]) -> IResult<&[u8], Vec<Frames>> {
  map(
    many_till(
      alt((padding, text_frame, generic_frame)), eof),
    |(frames, _)| frames)(input)
}

fn all_frames_v23(input: &[u8]) -> IResult<&[u8], Vec<Frames>> {
  map(
    many_till(
      alt((padding, text_frame_v23, generic_frame_v23)), eof),
    |(frames, _)| frames)(input)
}

fn text_frame_v23(input: &[u8]) -> IResult<&[u8], Frames> {
  alt((text_frame_utf8_v23, text_frame_utf16_v23))(input)
}

fn text_frame_utf8_v23(input: &[u8]) -> IResult<&[u8], Frames> {
  let (input, (_, id, size, flags)) = text_header_v23(input)?;
  let (input, (_, text)) =
    tuple((
      alt((tag(b"\x00"), tag(b"\x03"))),
      count(be_u8, (size - 1) as usize)
    ))(input)?;
  let text = String::from_utf8(text).unwrap().replace("\u{0}", "\n");
  debug!("utf8v23 {} {} {}", id, size, text);
  Ok((input, Frames::Text { id: id.to_string(), size, flags, text }))
}

fn text_frame_utf16_v23(input: &[u8]) -> IResult<&[u8], Frames> {
  let (input, (_, id, size, flags)) = text_header_v23(input)?;
  let (input, (_, text)) =
    tuple((
      tag(b"\x01\xff\xfe"),
      count(le_u16, (size - 3) as usize / 2)
    ))(input)?;
  let text = String::from_utf16(&*text).unwrap()
    .replace("\u{0000}", "\n").replace("\u{feff}", "");
  debug!("utf16v23 {} {} {}", id, size, text);
  Ok((input, Frames::Text { id: id.to_string(), size, flags, text }))
}

fn text_header_v23(input: &[u8]) -> IResult<&[u8], (&[u8], &str, u32, u16)> {
  tuple((
    tag("T"),
    map(
      take(3u8),
      |res| from_utf8(res).unwrap(),
    ),
    be_u32,
    be_u16
  ))(input)
}

fn generic_frame_v23(input: &[u8]) -> IResult<&[u8], Frames> {
  let (input, (id, size, flags)) =
    tuple((id_as_str, be_u32, be_u16))(input)?;
  debug!("frame {} {}", id, size);
  let (input, data) = take(size)(input)?;
  Ok((input, Frames::Frame { id: id.to_string(), size, flags, data: data.into() }))
}

pub fn find_energy(file: &str) -> Option<String> {
  let mut file = std::fs::File::open(file).unwrap();

  let mut buffer = [0; 10];
  file.read_exact(&mut buffer).unwrap();

  let (_, header) = file_header(&buffer).ok().unwrap();
  let mut input = vec![0u8; header.tag_size as usize];
  file.read_exact(&mut input).unwrap();

  let (_, result) = all_frames(&input).ok().unwrap();
  result.iter()
    .find(|f| match f {
      Frames::Text { id: _, size: _, flags: _, text } => text.starts_with("Energy"),
      _ => false
    }).map(|f| match f {
    Frames::Text { id: _, size: _, flags: _, text } => Some(text.to_string()),
    _ => None
  }).flatten()
}


pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct ID3Tag {
  pub filepath: String,
  pub frames: Vec<Frames>,
}

impl ID3Tag {
  pub fn read(filepath: &str) -> Result<ID3Tag> {
    let (mut file, header) = Self::read_header(filepath)?;
    let mut input = vec![0u8; header.tag_size as usize];
    file.read_exact(&mut input).unwrap();

    let (_, result) = if header.version == 3 {
      all_frames_v23(&input).map_err(|_| "Frames error")?
    } else {
      all_frames(&input).map_err(|_| "Frames error")?
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

  pub fn fix_copy_error_with_padding(filepath: &str) -> Result<()> {
    let mut file = OpenOptions::new().read(true).write(true).open(filepath)?;
    let mut buffer = [0; 10];
    file.read_exact(&mut buffer).unwrap();

    let (_, header) = file_header(&buffer).map_err(|_| "Header error")?;
    assert_eq!(header.version, 4);

    let mut buffer = [0; 3];
    file.seek(SeekFrom::Start(10 + header.tag_size as u64))?;
    file.read_exact(&mut buffer)?;
    if &buffer == b"\xFF\xFB\xE0" {
      debug!("Checking padding {}...", filepath);
      file.seek(SeekFrom::Start(header.tag_size as u64))?;
      let mut buffer = [0; 10];
      file.read_exact(&mut buffer)?;
      if &buffer != b"\0\0\0\0\0\0\0\0\0\0" {
        info!("Padding {}...", filepath);
        file.seek(SeekFrom::Start(header.tag_size as u64))?;
        let padding: [u8; 10] = [0; 10];
        file.write(padding.as_ref())?;
        file.sync_all()?
      }
    }

    Ok(())
  }

  pub fn write(&self, target: &str) -> Result<()> {
    let frames = self.frames.iter().filter(|f| match f {
      Frames::Text { id, .. } => {
        debug!("text {}", id);
        id != "FAIL"
      }
      Frames::Frame { id, .. } => {
        debug!("frame {}", id);
        id != "FAIL"
      }
      _ => false
    }).collect::<Vec<&Frames>>();

    let (mut file, header) = Self::read_header(&*self.filepath)?;

    let mut out = if self.filepath == target {
      let mut tmp = File::create("stream.tmp")?;
      file.seek(SeekFrom::Start(10 + header.tag_size as u64))?;
      let mut buffer = [0; 3];
      file.read_exact(&mut buffer)?;
      if &buffer != b"\xFF\xFB\xE0" {
        debug!("Trying earlier header");
        file.seek(SeekFrom::Start(header.tag_size as u64))?;
        file.read_exact(&mut buffer)?;
        if &buffer != b"\xFF\xFB\xE0" {
          error!("No MP3 header");
          return Ok(());
        }
        file.seek(SeekFrom::Start(header.tag_size as u64))?;
      } else {
        file.seek(SeekFrom::Start(10 + header.tag_size as u64))?;
      }
      std::io::copy(&mut file, &mut tmp)?;
      OpenOptions::new().write(true).open(&self.filepath)?
    } else {
      File::create(target)?
    };

    out.write(b"ID3\x04\x00\x00FAKE")?;

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
          if id == "XXX" {
            let string = text.replace("\n", "\u{0}");
            let text: &[u8] = string.as_bytes();
            let len = text.len() as u32 + 1;
            let vec = as_syncsafe(len);
            out.write(b"T")?;
            out.write(id.as_ref())?;
            out.write(&*vec)?;
            out.write(&flags.to_be_bytes())?;
            out.write(b"\x03")?;
            out.write(&*text)?;
          } else {
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
        }
        _ => {}
      }
    }

    let size = out.stream_position()? - 10;
    // tag size excludes header
    debug!("new tag size {}", size);
    let vec = as_syncsafe(size as u32);
    out.seek(SeekFrom::Start(6))?;
    out.write(&*vec)?;
    out.seek(SeekFrom::Start(10 + size))?;

    if self.filepath == target {
      let mut tmp = File::open("stream.tmp")?;
      std::io::copy(&mut tmp, &mut out)?;
    } else {
      file.seek(SeekFrom::Start(10 + header.tag_size as u64))?;
      std::io::copy(&mut file, &mut out)?;
    };

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

  pub fn extended_text(&self, description: &str) -> Option<String> {
    let terminated = format!("{}\n", description);
    self.frames.iter().find(|f| match f {
      Frames::Text { id, size: _, flags: _, text } => (id == "XXX" && text.starts_with(&terminated)),
      _ => false
    }).map(|f| match f {
      Frames::Text { id: _, size: _, flags: _, text } => Some(text[terminated.len()..].to_string()),
      _ => None
    }).flatten()
  }

  pub fn key(&self) -> Option<String> {
    self.text("KEY")
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

  pub fn set_title(&mut self, text: &str) {
    self.set_text("IT2", text);
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

  pub fn set_extended_text(&mut self, description: &str, value: &str) {
    if let Some(index) = self.frames.iter().position(|frame|
      match frame {
        Frames::Text { id, size: _, flags: _, text } => id == "XXX" && text.starts_with(description),
        _ => false
      }) {
      self.frames.remove(index);
    }
    self.frames.push(Frames::Text { id: "XXX".to_string(), size: 0, flags: 0, text: format!("{}\n{}", description, value) })
  }
}

fn as_syncsafe(total: u32) -> Vec<u8> {
  let mut result: Vec<u8> = Vec::new();
  let mut remaining = total;
  for _byte in total.to_be_bytes() {
    result.insert(0, (remaining & 0b01111111) as u8);
    remaining = remaining >> 7;
  }
  result
}

fn as_syncsafe_bytes(total: u32) -> u32 {
  let vec = as_syncsafe(total);
  let (bytes, _) = vec.as_slice().split_at(std::mem::size_of::<u32>());
  u32::from_be_bytes(bytes.try_into().unwrap())
}

pub fn log_init() {
  let _ = env_logger::builder().is_test(true)
    .filter_level(LevelFilter::Debug)
    .try_init();
}

#[cfg(test)]
mod tests {
  use std::fs;
  use std::fs::File;
  use std::io::Read;

  use assert_matches::assert_matches;

  use super::*;

  #[test]
  pub fn test_sum() {
    log_init();

    let tag = ID3Tag::read("oil-rigger-v24-ro.mp3").unwrap();
    let sum = tag.frames.iter()
      .fold(0u32, |sum, frame| sum + match frame {
        Frames::Frame { id: _, size, flags: _, data: _ } => (10 + size),
        Frames::Text { id: _, size, flags: _, text: _ } => (10 + size),
        Frames::Padding { size } => (0 + size),
      });

    // [2021-10-19T18:21:38Z DEBUG id3_rs] utf16 PE1 15
    // [2021-10-19T18:21:38Z DEBUG id3_rs] utf16 IT2 23
    // [2021-10-19T18:21:38Z DEBUG id3_rs] utf16 ALB 11
    // [2021-10-19T18:21:38Z DEBUG id3_rs] utf16 IT3 3
    // [2021-10-19T18:21:38Z DEBUG id3_rs] utf16 CON 15

    assert_eq!(sum, 66872);

    let _sum = tag.frames.iter()
      .fold(0u32, |sum, frame| sum + match frame {
        Frames::Frame { id: _, size, flags: _, data: _ } => (10 + size),
        Frames::Text { id: _, size: _, flags: _, text } => (10 + 1 + text.len() as u32),
        Frames::Padding { size } => (0 + size),
      });

    let _double_utf16 = 15 + 23 + 11 + 3 + 15 + (5 * 2); // 67
  }

  #[test]
  pub fn test_frames() {
    log_init();

    let tag = ID3Tag::read("oil-rigger-v24-ro.mp3").unwrap();
    assert_eq!(tag.frames.len(), 17);
  }

  #[test]
  pub fn test_reading() {
    log_init();

    let tag = ID3Tag::read("/Users/bas/OneDrive/PioneerDJ/liked/0. Rain -- Komputer [73160775].mp3").unwrap();
    assert_eq!(tag.frames.len(), 20);
  }

  #[test]
  pub fn test_change_copy() {
    log_init();

    let mut tag = ID3Tag::read("oil-rigger-v24-ro.mp3").unwrap();
    tag.set_title("Roil Igger");
    tag.set_extended_text("EnergyLevel", "99");
    tag.write("oil-rigger-out.mp3").unwrap();
  }

  #[test]
  pub fn test_extended_text() {
    log_init();

    let mut tag = ID3Tag::read("oil-rigger-v24-ro.mp3").unwrap();
    tag.set_extended_text("OriginalTitle", "Oil Rigger");
    assert_eq!(tag.extended_text("OriginalTitle"), Some("Oil Rigger".to_string()));
  }

  #[test]
  pub fn test_23_extended_text() {
    log_init();

    let tag = ID3Tag::read("london-bass-v23-ro.mp3").unwrap();
    assert_eq!(tag.extended_text("OriginalTitle"), Some("London Bass".to_string()));
  }

  #[test]
  pub fn test_energy_level() {
    log_init();

    let tag = ID3Tag::read("energy-utf7-ro.mp3").unwrap();
    assert_eq!(tag.extended_text("EnergyLevel"), Some("6".to_string()));
    let tag = ID3Tag::read("energy-utf16-ro.mp3").unwrap();
    assert_eq!(tag.extended_text("EnergyLevel"), Some("6".to_string()));
  }

  #[test]
  pub fn test_title() {
    log_init();

    let tag = ID3Tag::read("/Users/bas/OneDrive/PioneerDJ/minimal/0. Applause -- Rossi. [901764012].mp3").unwrap();
    assert_eq!(tag.title(), Some(" 7a E  Applause".to_string()));
  }

  #[test]
  pub fn test_repairing_copy() {
    log_init();

    fs::copy("fixing-ro.mp3", "fixing-rw.mp3").unwrap();
    let tag = ID3Tag::read("fixing-ro.mp3").unwrap();
    tag.write("fixing-repair.mp3").unwrap();
  }

  #[test]
  pub fn test_repairing_inplace() {
    log_init();

    fs::copy("fixing-ro.mp3", "fixing-rw.mp3").unwrap();
    let tag = ID3Tag::read("fixing-rw.mp3").unwrap();
    tag.write("fixing-rw.mp3").unwrap();
  }

  #[test]
  pub fn test_change_inplace() {
    log_init();

    fs::copy("oil-rigger-v24-ro.mp3", "oil-rigger-rw.mp3").unwrap();
    let mut tag = ID3Tag::read("oil-rigger-v24-ro.mp3").unwrap();
    tag.set_title("Roil Igger");
    tag.set_extended_text("EnergyLevel", "99");
    tag.write("oil-rigger-rw.mp3").unwrap();
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

  #[test]
  pub fn test_class_text() {
    let tag = ID3Tag::read("oil-rigger-v24-ro.mp3").unwrap();

    assert_eq!(tag.text("IT2"), Some("Oil Rigger".to_string()));
    assert_eq!(tag.extended_text("EnergyLevel"), Some("6".to_string()));
    assert_eq!(tag.title(), Some("Oil Rigger".to_string()));
    assert_eq!(tag.subtitle(), Some("".to_string()));
    assert_eq!(tag.key(), Some("4A".to_string()));
    assert_eq!(tag.artist(), Some("Regent".to_string()));
  }

  #[test]
  pub fn test_library() {
    log_init();
    let tag = ID3Tag::read("oil-rigger-v24-ro.mp3").unwrap();

    assert_eq!(tag.text("IT2"), Some("Oil Rigger".to_string()));
    assert_eq!(tag.extended_text("EnergyLevel"), Some("6".to_string()));
    assert_eq!(tag.extended_text("OriginalTitle"), None);
    assert_eq!(tag.title(), Some("Oil Rigger".to_string()));
    assert_eq!(tag.subtitle(), Some("".to_string()));
    assert_eq!(tag.key(), Some("4A".to_string()));
    assert_eq!(tag.artist(), Some("Regent".to_string()));
  }

  fn get_test_file() -> File {
    let filepath = "oil-rigger-v24-ro.mp3";
    let file = std::fs::File::open(filepath).unwrap();
    file
  }

  #[test]
  fn test_energy() {
    assert_eq!(find_energy("oil-rigger-v24-ro.mp3"), Some("EnergyLevel\n6".to_string()));
  }

  #[test]
  fn test_header_and_frames() {
    let mut file = get_test_file();
    let mut buffer = [0; 10];
    file.read_exact(&mut buffer).unwrap();

    let (_, header) = file_header(&buffer).ok().unwrap();
    assert_eq!(header, Header { version: 4, revision: 0, flags: 0, tag_size: 66872 });

    let mut input = vec![0u8; header.tag_size as usize];
    file.read_exact(&mut input).unwrap();

    let (_, result) = all_frames(&input).ok().unwrap();
    assert_eq!(17, result.len());
  }

  #[test]
  fn test_offset_fix() {
    let filepath = "/Users/bas/OneDrive/PioneerDJ/discover/DW202141/24. Positive Energy Forever -- Mall Grab [1386413292].mp3";
    let mut file = OpenOptions::new().read(true).write(true).open(filepath).unwrap();
    let mut buffer = [0; 10];
    file.read_exact(&mut buffer).unwrap();

    let (_, header) = file_header(&buffer).ok().unwrap();
    assert_eq!(header, Header { version: 4, revision: 0, flags: 0, tag_size: 101535 });

    file.seek(SeekFrom::Start(10 + header.tag_size as u64)).unwrap();
    file.read_exact(&mut buffer).unwrap();
    if &buffer == b"\xFF\xFB\xE0\0\0\0\0\0\0\0" {
      debug!("Checking padding {}...", filepath);
      file.seek(SeekFrom::Start(header.tag_size as u64)).unwrap();
      file.read_exact(&mut buffer).unwrap();
      if &buffer == b"\0\0\0\0\0\0\0\0\0\0" {
        info!("Padding {}...", filepath);
        file.seek(SeekFrom::Start(header.tag_size as u64)).unwrap();
        let padding: [u8; 10] = [0; 10];
        file.write(padding.as_ref()).unwrap();
      }
    }
  }

  #[test]
  fn test_frames_individually() {
    log_init();

    let mut file = get_test_file();
    let mut buffer = [0; 10];
    file.read_exact(&mut buffer).unwrap();

    let (_, header) = file_header(&buffer).ok().unwrap();
    assert_eq!(header, Header { version: 4, revision: 0, flags: 0, tag_size: 66872 });

    let mut input = vec![0u8; header.tag_size as usize];
    file.read_exact(&mut input).unwrap();

    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "PE1".to_string(), size: 15, flags: 0, text: "Regent".to_string() });
    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "IT2".to_string(), size: 23, flags: 0, text: "Oil Rigger".to_string() });
    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "ALB".to_string(), size: 11, flags: 0, text: "Nova".to_string() });
    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "IT3".to_string(), size: 3, flags: 0, text: "".to_string() });
    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "CON".to_string(), size: 15, flags: 0, text: "techno".to_string() });

    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id, size: 40952, flags: _, data: _} => {
      assert_eq!(id, "APIC".to_string());
      // TODO: compare actual picture
      // if let Frames::Frame { id, size, flags, data } = frame {
      //   let mut out = File::create("APIC.bin").unwrap();
      //   out.write(data).unwrap();
      // }
    });

    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id, size: 557, flags: _, data: _}=> {
      assert_eq!(id, "GEOB".to_string());
    });

    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id, size: 353, flags: _, data: _}=> {
      assert_eq!(id, "GEOB".to_string());
    });

    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id, size: 321, flags: _, data: _}=> {
      assert_eq!(id, "GEOB".to_string());
    });

    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id, size: 11, flags: _, data: _}=> {
      assert_eq!(id, "COMM".to_string());
    });

    //         4A
    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "KEY".to_string(), size: 3, flags: 0, text: "4A".to_string() });

    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "BPM".to_string(), size: 4, flags: 0, text: "142".to_string() });

    //      
    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "XXX".to_string(), size: 14, flags: 0, text: "EnergyLevel\n6".to_string() });

    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id, size: 92, flags: _, data: _}=> {
      assert_eq!(id, "GEOB".to_string());
    });

    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id, size: 100, flags: _, data: _}=> {
      assert_eq!(id, "GEOB".to_string());
    });

    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id, size: 23214, flags: _, data: _}=> {
      assert_eq!(id, "GEOB".to_string());
    });

    let (_input, frame) = padding(&input).ok().unwrap();
    assert_matches!(frame, Frames::Padding{ size: 1024});
  }
}