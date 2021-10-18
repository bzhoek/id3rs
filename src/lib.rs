use std::str::from_utf8;

use nom::branch::alt;
use nom::bytes::streaming::{tag, take};
use nom::combinator::{eof, map};
use nom::IResult;
use nom::multi::{fold_many_m_n, many_m_n, many_till};
use nom::number::complete::be_u32;
use nom::number::streaming::{be_u16, be_u8, le_u16};
use nom::sequence::tuple;

#[derive(Debug, PartialEq, Eq)]
pub struct Header {
  version: u8,
  revision: u8,
  flags: u8,
  tag_size: u64,
}

#[derive(Debug, PartialEq, Eq)]
enum Frames<'a> {
  Frame {
    id: &'a str,
    size: u32,
    flags: u16,
    data: Vec<u8>
  },
  Text {
    id: &'a str,
    size: u32,
    flags: u16,
    text: String,
  },
}

#[derive(Debug, PartialEq, Eq)]
pub struct Frame<'a> {
  id: &'a str,
  size: u32,
  flags: u16,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Text<'a> {
  id: &'a str,
  size: u32,
  flags: u16,
  text: String,
}

fn file_header(input: &[u8]) -> IResult<&[u8], Header> {
  let (input, (_, version, revision, flags, next)) = tuple(
    (tag("ID3"),
      be_u8,
      be_u8,
      be_u8,
      fold_many_m_n(4, 4, be_u8, 0u64, |acc, byte| acc << 7 | (byte as u64))
    ))(input)?;
  Ok((input, Header { version, revision, flags, tag_size: next }))
}

fn id_as_str(input: &[u8]) -> IResult<&[u8], &str> {
  map(
    take(4u8),
    |res| from_utf8(res).unwrap(),
  )(input)
}

fn generic_frame(input: &[u8]) -> IResult<&[u8], Frames> {
  let (input, (id, size, flags)) = tuple((
    id_as_str,
    be_u32,
    be_u16
  ))(input)?;
  let (input, _) = take(size)(input)?;
  Ok((input, Frames::Frame { id, size, flags }))
}

fn text_frame(input: &[u8]) -> IResult<&[u8], Frames> {
  let (input, (_, id, size, flags)) = tuple((
    tag("T"),
    map(
      take(3u8),
      |res| from_utf8(res).unwrap(),
    ),
    be_u32,
    be_u16
  ))(input)?;
  let words = (size - 3) as usize / 2;
  let (input, (_, text)) = tuple((
    tag(b"\x01\xff\xfe"),
    many_m_n(words, words, le_u16)
  ))(input)?;
  Ok((input, Frames::Text { id, size, flags, text: String::from_utf16(&*text).unwrap() }))
}

fn frames(input: &[u8]) -> IResult<&[u8], Vec<Frames>> {
  map(
    many_till(alt((text_frame, generic_frame)), eof),
    |(frames, _)| frames)(input)
}


#[cfg(test)]
mod tests {
  use std::io::Read;

  use super::*;

  #[test]
  fn test_open() {
    let filepath = "/Users/bas/OneDrive/PioneerDJ/techno/53. Semantic Drift  -- Dustin Zahn [1196743132].mp3";
    let mut file = std::fs::File::open(filepath).unwrap();
    let mut buffer = [0; 10];
    file.read_exact(&mut buffer).unwrap();

    let (_, header) = file_header(&buffer).ok().unwrap();
    assert_eq!(header, Header { version: 3, revision: 0, flags: 0, tag_size: 46029 });

    let mut input = vec![0u8; header.tag_size as usize];
    file.read_exact(&mut input).unwrap();

    let (_, result) = frames(&input).ok().unwrap();
    assert_eq!(9, result.len());
  }

  #[test]
  fn test_one_by_one() {
    let filepath = "/Users/bas/OneDrive/PioneerDJ/techno/53. Semantic Drift  -- Dustin Zahn [1196743132].mp3";
    let mut file = std::fs::File::open(filepath).unwrap();
    let mut buffer = [0; 10];
    file.read_exact(&mut buffer).unwrap();

    let (_, header) = file_header(&buffer).ok().unwrap();
    assert_eq!(header, Header { version: 3, revision: 0, flags: 0, tag_size: 46029 });

    let mut input = vec![0u8; header.tag_size as usize];
    file.read_exact(&mut input).unwrap();

    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "ALB", size: 39, flags: 0, text: "The Shock Doctrine".to_string() });
    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "PE1", size: 25, flags: 0, text: "Dustin Zahn".to_string() });
    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "IT2", size: 47, flags: 0, text: " 9a E  Semantic Drift ".to_string() });
    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Frame { id: "APIC", size: 45750, flags: 0 });
    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "IT3", size: 3, flags: 0, text: "".to_string() });
    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "BPM", size: 9, flags: 0, text: "128".to_string() });
    let (input, frame) = generic_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Frame { id: "COMM", size: 22, flags: 0 });
    let (input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "XXX", size: 21, flags: 0, text: "Rating\u{0}\u{feff}3".to_string() });
    let (_input, frame) = text_frame(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "CON", size: 23, flags: 0, text: "techno/bup".to_string() });
  }
}