use std::str::from_utf8;

use log::debug;
use nom::branch::alt;
use nom::bytes::streaming::{tag, take};
use nom::combinator::{eof, map};
use nom::IResult;
use nom::multi::{fold_many_m_n, many_till};
use nom::number::complete::be_u32;
use nom::number::streaming::{be_u16, be_u8, le_u16, le_u8};
use nom::sequence::{pair, tuple};

use crate::{Frames, Header};

fn id_as_str(input: &[u8]) -> IResult<&[u8], &str> {
  map(
    take(4u8),
    |res| from_utf8(res).unwrap(),
  )(input)
}

fn data_size_v24(input: &[u8]) -> IResult<&[u8], u32> {
  fold_many_m_n(4, 4, be_u8, 0u32,
    |acc, byte| acc << 7 | (byte as u32))(input)
}

fn data_size_v23(input: &[u8]) -> IResult<&[u8], u32> {
  be_u32(input)
}

pub fn all_frames_v23(input: &[u8]) -> IResult<&[u8], Vec<Frames>> {
  map(
    many_till(
      alt((padding, extended_text_frame_v23, text_frame_v23, object_frame_v23, generic_frame_v23)), eof),
    |(frames, _)| frames)(input)
}

pub fn all_frames_v24(input: &[u8]) -> IResult<&[u8], Vec<Frames>> {
  map(
    many_till(
      alt((padding, extended_text_frame_v24, text_frame_v24, object_frame_v24, generic_frame_v24)), eof),
    |(frames, _)| frames)(input)
}

fn extended_text_frame_v23(input: &[u8]) -> IResult<&[u8], Frames> {
  extended_text_frame(input, data_size_v23)
}

fn extended_text_frame_v24(input: &[u8]) -> IResult<&[u8], Frames> {
  extended_text_frame(input, data_size_v24)
}

fn extended_text_frame(input: &[u8], data_size: fn(&[u8]) -> IResult<&[u8], u32>) -> IResult<&[u8], Frames> {
  let (input, (id, size, flags)) = tuple((tag("TXXX"), data_size, be_u16))(input)?;
  let id = from_utf8(id).unwrap().to_string();
  debug!("extended {}", id);
  let (input, (encoding, data)) = pair(be_u8, take(size - 1))(input)?;
  let (_data, (description, value)) = encoded_string_pair(encoding, data)?;
  debug!("extended {} value {}", description, value);
  Ok((input, Frames::ExtendedText { id, size, flags, description, value }))
}

fn encoded_string_pair(encoding: u8, data: &[u8]) -> IResult<&[u8], (String, String)> {
  match encoding {
    1 => { tuple((terminated_utf16, terminated_utf16))(data) }
    _ => { tuple((terminated_utf8, terminated_utf8))(data) }
  }
}

fn encoded_string(encoding: u8, data: &[u8]) -> IResult<&[u8], String> {
  match encoding {
    1 => { terminated_utf16(data) }
    _ => { terminated_utf8(data) }
  }
}

fn text_frame_v23(input: &[u8]) -> IResult<&[u8], Frames> {
  text_frame(input, data_size_v23)
}

fn text_frame_v24(input: &[u8]) -> IResult<&[u8], Frames> {
  text_frame(input, data_size_v24)
}

fn text_frame(input: &[u8], data_size: fn(&[u8]) -> IResult<&[u8], u32>) -> IResult<&[u8], Frames> {
  let (input, (_, id, size, flags)) =
    tuple((
      tag("T"),
      map(
        take(3u8),
        |res| from_utf8(res).unwrap(),
      ),
      data_size, be_u16))(input)?;
  let (input, (encoding, data)) = pair(be_u8, take(size - 1))(input)?;
  let (_data, text) = encoded_string(encoding, data)?;
  debug!("utf8v23 {} {} {}", id, size, text);
  Ok((input, Frames::Text { id: id.to_string(), size, flags, text }))
}

fn generic_frame_v23(input: &[u8]) -> IResult<&[u8], Frames> {
  generic_frame(input, data_size_v23)
}

fn generic_frame_v24(input: &[u8]) -> IResult<&[u8], Frames> {
  generic_frame(input, data_size_v24)
}

fn generic_frame(input: &[u8], data_size: fn(&[u8]) -> IResult<&[u8], u32>) -> IResult<&[u8], Frames> {
  let (input, (id, size, flags)) =
    tuple((id_as_str, data_size, be_u16))(input)?;
  debug!("frame {} {}", id, size);
  let (input, data) = take(size)(input)?;
  Ok((input, Frames::Frame { id: id.to_string(), size, flags, data: data.into() }))
}

fn object_frame_v23(input: &[u8]) -> IResult<&[u8], Frames> {
  object_frame(input, data_size_v23)
}

fn object_frame_v24(input: &[u8]) -> IResult<&[u8], Frames> {
  object_frame(input, data_size_v24)
}

fn object_frame(input: &[u8], data_size: fn(&[u8]) -> IResult<&[u8], u32>) -> IResult<&[u8], Frames> {
  let (input, (id, size, flags)) = tuple((tag("GEOB"), data_size, be_u16))(input)?;
  let id = from_utf8(id).unwrap().to_string();
  debug!("object {:?} {}",  id, size);
  let offset = input.len();
  let (input, encoding) = be_u8(input)?;
  let (input, mime_type) = terminated_utf8(input)?;
  let (input, (filename, description)) = encoded_string_pair(encoding, input)?;
  let remaining = size - (offset - input.len()) as u32;
  debug!("mime {}, filename {}, size {}, description {}", mime_type, filename, remaining, description);
  let (input, data) = take(remaining)(input)?;
  Ok((input, Frames::Object { id, size, flags, mime_type, filename, description, data: data.into() }))
}

fn terminated_utf8(input: &[u8]) -> IResult<&[u8], String> {
  let (input, bytes) = many_till(le_u8, alt((eof, tag(b"\x00"))))(input)?;
  let text = String::from_utf8(bytes.0).unwrap();
  debug!("utf8 {}", text);
  Ok((input, text))
}

fn terminated_utf16(input: &[u8]) -> IResult<&[u8], String> {
  let (input, _bom) = tag(b"\xff\xfe")(input)?;
  let (input, (words, _nul)) = many_till(le_u16, alt((eof, tag(b"\x00\x00"))))(input)?;

  let text = String::from_utf16(&words).unwrap();
  debug!("utf16 {}", text);
  Ok((input, text))
}

pub fn file_header(input: &[u8]) -> IResult<&[u8], Header> {
  let (input, (_, version, revision, flags, tag_size))
    = tuple((tag("ID3"), be_u8, be_u8, be_u8, data_size_v24))(input)?;
  debug!("ID3 {} tag size {}", version, tag_size);
  Ok((input, Header { version, revision, flags, tag_size }))
}

fn padding(input: &[u8]) -> IResult<&[u8], Frames> {
  let (input, pad) =
    many_till(tag(b"\x00"), eof)
      (input)?;
  Ok((input, Frames::Padding { size: pad.0.len() as u32 }))
}

pub fn as_syncsafe(total: u32) -> Vec<u8> {
  let mut result: Vec<u8> = Vec::new();
  let mut remaining = total;
  for _byte in total.to_be_bytes() {
    result.insert(0, (remaining & 0b01111111) as u8);
    remaining = remaining >> 7;
  }
  result
}

#[cfg(test)]
mod tests {
  use std::io::Read;

  use assert_matches::assert_matches;

  use crate::log_init;

  use super::*;

  #[test]
  fn test_header_and_frames() {
    let (rofile, _, _) = filenames("4bleak");
    let mut file = std::fs::File::open(&rofile).unwrap();
    let mut buffer = [0; 10];
    file.read_exact(&mut buffer).unwrap();

    let (_, header) = file_header(&buffer).ok().unwrap();
    assert_eq!(header, Header { version: 4, revision: 0, flags: 0, tag_size: 42316 });

    let mut input = vec![0u8; header.tag_size as usize];
    file.read_exact(&mut input).unwrap();

    let (_, result) = all_frames_v24(&input).ok().unwrap();
    assert_eq!(14, result.len());
  }

  #[test]
  fn test_frames_individually() {
    log_init();

    let (rofile, _, _) = filenames("4bleak");
    let mut file = std::fs::File::open(&rofile).unwrap();
    let mut buffer = [0; 10];
    file.read_exact(&mut buffer).unwrap();

    let (_, header) = file_header(&buffer).ok().unwrap();
    assert_eq!(header, Header { version: 4, revision: 0, flags: 0, tag_size: 42316 });

    let mut input = vec![0u8; header.tag_size as usize];
    file.read_exact(&mut input).unwrap();

    let (input, frame) = text_frame_v24(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "PE1".to_string(), size: 25, flags: 0, text: "Maenad Veyl".to_string() });
    let (input, frame) = text_frame_v24(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "IT2".to_string(), size: 13, flags: 0, text: "Bleak".to_string() });
    let (input, frame) = text_frame_v24(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "ALB".to_string(), size: 23, flags: 0, text: "Body Count".to_string() });
    let (input, frame) = text_frame_v24(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "IT3".to_string(), size: 3, flags: 0, text: "".to_string() });

    let (input, frame) = generic_frame_v24(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id, size: 26524, flags: _, data: _} => {
      assert_eq!(id, "APIC".to_string());
      // TODO: compare actual picture
      // if let Frames::Frame { id, size, flags, data } = frame {
      //   let mut out = File::create("APIC.bin").unwrap();
      //   out.write(data).unwrap();
      // }
    });

    let (input, frame) = generic_frame_v24(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id, size: 11, flags: _, data: _}=> {
      assert_eq!(id, "COMM".to_string());
    });

    //         4A
    let (input, frame) = text_frame_v24(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "KEY".to_string(), size: 3, flags: 0, text: "4A".to_string() });

    let (input, frame) = text_frame_v24(&input).ok().unwrap();
    assert_eq!(frame, Frames::Text { id: "BPM".to_string(), size: 4, flags: 0, text: "100".to_string() });

    //      
    let (input, frame) = extended_text_frame_v24(&input).ok().unwrap();
    assert_eq!(frame, Frames::ExtendedText { id: "TXXX".to_string(), size: 14, flags: 0, description: "EnergyLevel".to_string(), value: "6".to_string() });

    let (input, frame) = generic_frame_v24(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id, size: 92, flags: _, data: _}=> {
      assert_eq!(id, "GEOB".to_string());
    });

    let (input, frame) = generic_frame_v24(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id, size: 100, flags: _, data: _}=> {
      assert_eq!(id, "GEOB".to_string());
    });

    let (input, frame) = generic_frame_v24(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id, size: 13789, flags: _, data: _}=> {
      assert_eq!(id, "GEOB".to_string());
    });

    let (_input, frame) = generic_frame_v24(&input).ok().unwrap();
    assert_matches!(frame, Frames::Frame{ id, size: 561, flags: _, data: _}=> {
      assert_eq!(id, "GEOB".to_string());
    });
  }

  fn filenames(base: &str) -> (String, String, String) {
    (format!("{}.mp3", base), format!("{}-out.mp3", base), format!("{}-rw.mp3", base))
  }
}