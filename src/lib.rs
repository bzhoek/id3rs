use std::str::from_utf8;

use nom::bytes::streaming::{tag, take};
use nom::combinator::map;
use nom::IResult;
use nom::multi::fold_many_m_n;
use nom::number::complete::be_u32;
use nom::number::streaming::{be_u16, be_u8};
use nom::sequence::tuple;

#[derive(Debug, PartialEq, Eq)]
pub struct Header {
  version: u8,
  revision: u8,
  flags: u8,
  tag_size: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Frame<'a> {
  id: &'a str,
  size: u32,
  flags: u16,
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

fn frame(input: &[u8]) -> IResult<&[u8], Frame> {
  let (input, (id, size, flags)) = tuple((
    id_as_str,
    be_u32,
    be_u16
  ))(input)?;
  Ok((input, Frame { id, size, flags }))
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

    let mut tag = vec![0u8; header.tag_size as usize];
    file.read_exact(&mut tag).unwrap();

    let (_, frame) = frame(&tag).ok().unwrap();
    assert_eq!(frame, Frame { id: "TALB", size: 39, flags: 0 });
  }
}