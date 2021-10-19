#[macro_use]
extern crate assert_matches;

use std::io::Read;
use std::str::from_utf8;

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
enum Frames<'a> {
  Frame {
    id: &'a str,
    size: u32,
    flags: u16,
    data: &'a [u8],
  },
  Text {
    id: &'a str,
    size: u32,
    flags: u16,
    text: String,
  },
  Padding {
    size: u32
  },
}

fn file_header(input: &[u8]) -> IResult<&[u8], Header> {
  let (input, (_, version, revision, flags, next))
    = tuple((tag("ID3"), be_u8, be_u8, be_u8, syncsafe))(input)?;
  Ok((input, Header { version, revision, flags, tag_size: next }))
}

fn id_as_str(input: &[u8]) -> IResult<&[u8], &str> {
  map(
    take(4u8),
    |res| from_utf8(res).unwrap(),
  )(input)
}

fn syncsafe(input: &[u8]) -> IResult<&[u8], u32> {
  fold_many_m_n(4, 4, be_u8, 0u32,
    |acc, byte| acc << 7 | (byte as u32))(input)
}

fn generic_frame(input: &[u8]) -> IResult<&[u8], Frames> {
  let (input, (id, size, flags)) =
    tuple((id_as_str, syncsafe, be_u16))(input)?;
  let (input, data) = take(size)(input)?;
  Ok((input, Frames::Frame { id, size, flags, data }))
}

fn text_header(input: &[u8]) -> IResult<&[u8], (&[u8], &str, u32, u16)> {
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

fn text_frame_utf16(input: &[u8]) -> IResult<&[u8], Frames> {
  let (input, (_, id, size, flags)) = text_header(input)?;
  let (input, (_, text)) =
    tuple((
      tag(b"\x01\xff\xfe"),
      count(le_u16, (size - 3) as usize / 2)
    ))(input)?;
  Ok((input, Frames::Text { id, size, flags, text: String::from_utf16(&*text).unwrap() }))
}

fn text_frame_utf8(input: &[u8]) -> IResult<&[u8], Frames> {
  let (input, (_, id, size, flags)) = text_header(input)?;
  let (input, (_, text)) =
    tuple((
      alt((tag(b"\x00"), tag(b"\x03"))),
      count(be_u8, (size - 1) as usize)
    ))(input)?;
  Ok((input, Frames::Text { id, size, flags, text: String::from_utf8(text).unwrap().replace("\u{0}", "\n") }))
}

fn text_frame(input: &[u8]) -> IResult<&[u8], Frames> {
  alt((text_frame_utf16, text_frame_utf8))(input)
}

fn padding(input: &[u8]) -> IResult<&[u8], Frames> {
  let (input, pad) =
    many_till(tag(b"\x00"), eof)
      (input)?;
  Ok((input, Frames::Padding { size: pad.0.len() as u32 }))
}

fn frames(input: &[u8]) -> IResult<&[u8], Vec<Frames>> {
  map(
    many_till(
      alt((padding, text_frame, generic_frame)), eof),
    |(frames, _)| frames)(input)
}

pub fn find_energy(file: &str) -> Option<String> {
  let mut file = std::fs::File::open(file).unwrap();

  let mut buffer = [0; 10];
  file.read_exact(&mut buffer).unwrap();

  let (_, header) = file_header(&buffer).ok().unwrap();
  let mut input = vec![0u8; header.tag_size as usize];
  file.read_exact(&mut input).unwrap();

  let (_, result) = frames(&input).ok().unwrap();
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

pub struct ID3Tag {}

impl ID3Tag {
  pub fn read() -> Result<ID3Tag> {
    Ok(ID3Tag {})
  }
}

#[cfg(test)]
mod tests {
  use std::fs::File;
  use std::io::Read;

  use super::*;

  #[test]
  pub fn test_class() {
    let tag = ID3Tag::read().unwrap();
  }

  fn get_test_file() -> File {
    let filepath = "13. Oil Rigger -- Regent [1506153642].mp3";
    let file = std::fs::File::open(filepath).unwrap();
    file
  }

  #[test]
  fn test_energy() {
    assert_eq!(find_energy("13. Oil Rigger -- Regent [1506153642].mp3"), Some("EnergyLevel\n6".to_string()));
  }

  #[test]
  fn test_frames() {
    let mut file = get_test_file();
    let mut buffer = [0; 10];
    file.read_exact(&mut buffer).unwrap();

    let (_, header) = file_header(&buffer).ok().unwrap();
    assert_eq!(header, Header { version: 4, revision: 0, flags: 0, tag_size: 66872 });

    let mut input = vec![0u8; header.tag_size as usize];
    file.read_exact(&mut input).unwrap();

    let (_, result) = frames(&input).ok().unwrap();
    assert_eq!(17, result.len());
  }

  #[test]
  fn test_frames_individually() {
    let mut file = get_test_file();
    let mut buffer = [0; 10];
    file.read_exact(&mut buffer).unwrap();

    let (_, header) = file_header(&buffer).ok().unwrap();
    assert_eq!(header, Header { version: 4, revision: 0, flags: 0, tag_size: 66872 });

    let mut input = vec![0u8; header.tag_size as usize];
    file.read_exact(&mut input).unwrap();

    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "PE1", size: 15, flags: 0, text: "Regent".to_string() });
    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "IT2", size: 23, flags: 0, text: "Oil Rigger".to_string() });
    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "ALB", size: 11, flags: 0, text: "Nova".to_string() });
    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "IT3", size: 3, flags: 0, text: "".to_string() });
    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "CON", size: 15, flags: 0, text: "techno".to_string() });

    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id: "APIC", size: 40952, flags: _, data: _} => {
      // TODO: compare actual picture
      // if let Frames::Frame { id, size, flags, data } = frame {
      //   let mut out = File::create("APIC.bin").unwrap();
      //   out.write(data).unwrap();
      // }
    });

    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id: "GEOB", size: 557, flags: _, data: _});

    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id: "GEOB", size: 353, flags: _, data: _});

    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id: "GEOB", size: 321, flags: _, data: _});

    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id: "COMM", size: 11, flags: _, data: _});

    //         4A
    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "KEY", size: 3, flags: 0, text: "4A".to_string() });

    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "BPM", size: 4, flags: 0, text: "142".to_string() });

    //      
    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "XXX", size: 14, flags: 0, text: "EnergyLevel\n6".to_string() });

    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id: "GEOB", size: 92, flags: _, data: _});

    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id: "GEOB", size: 100, flags: _, data: _});

    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id: "GEOB", size: 23214, flags: _, data: _});

    let (_input, frame) = padding(&input).ok().unwrap();
    assert_matches!(frame, Frames::Padding{ size: 1024});
  }
}