use std::str::from_utf8;

use log::debug;
use nom::branch::alt;
use nom::bytes::streaming::{tag, take};
use nom::character::streaming::one_of;
use nom::combinator::{eof, map};
use nom::IResult;
use nom::multi::{fold_many_m_n, many_till};
use nom::number::complete::be_u32;
use nom::number::streaming::{be_u16, be_u8, le_u16, le_u8};
use nom::sequence::{pair, tuple};

use crate::{COMMENT_TAG, EXTENDED_TAG, Frame, Header, OBJECT_TAG, PICTURE_TAG, POPULARITY_TAG};

fn id_as_str(input: &[u8]) -> IResult<&[u8], &str> {
  map(
    take(4u8),
    |res| from_utf8(res).unwrap(),
  )(input)
}

pub fn v24_len(input: &[u8]) -> IResult<&[u8], u32> {
  fold_many_m_n(4, 4, be_u8, || 0u32,
    |acc, byte| acc << 7 | (byte as u32))(input)
}

pub fn v23_len(input: &[u8]) -> IResult<&[u8], u32> {
  be_u32(input)
}

pub fn all_frames(len: fn(&[u8]) -> IResult<&[u8], u32>)
  -> impl FnMut(&[u8])
    -> IResult<&[u8], Vec<Frame>> {
  move |input| {
    map(
      many_till(alt((
        padding,
        extended_text_frame(len),
        comment_frame(len),
        object_frame(len),
        picture_frame(len),
        text_frame(len),
        popularity_frame(len),
        generic_frame(len))),
        eof),
      |(frames, _)| frames)(input)
  }
}

pub fn padding(input: &[u8]) -> IResult<&[u8], Frame> {
  let (input, pad) =
    many_till(tag(b"\x00"), eof)
      (input)?;
  debug!("Padding: {}", pad.0.len());
  Ok((input, Frame::Padding { size: pad.0.len() as u32 }))
}

pub fn extended_text_frame(len: fn(&[u8]) -> IResult<&[u8], u32>)
  -> impl FnMut(&[u8])
    -> IResult<&[u8], Frame> {
  move |input| {
    let (input, (id, size, flags)) = tuple((tag(EXTENDED_TAG), len, be_u16))(input)?;
    let id = from_utf8(id).unwrap().to_string();
    debug!("extended {}", id);
    let (input, (encoding, data)) = pair(be_u8, take(size - 1))(input)?;
    let (_data, (description, value)) = encoded_string_pair(encoding, data)?;
    debug!("extended {} value {}", description, value);
    Ok((input, Frame::ExtendedText { id, size, flags, description, value }))
  }
}

pub fn comment_frame(len: fn(&[u8]) -> IResult<&[u8], u32>)
  -> impl FnMut(&[u8])
    -> IResult<&[u8], Frame> {
  move |input| {
    let (input, (_id, size, flags, encoding, language)) =
      tuple((
        tag(COMMENT_TAG),
        len,
        be_u16,
        be_u8,
        map(
          take(3u8),
          |res| from_utf8(res).unwrap(),
        ),
      ))(input)?;
    let (input, data) = take(size - 4)(input)?;
    let (_data, (description, value)) = encoded_string_pair(encoding, data)?;
    debug!("comment {} {} {} {}", size, language, description, value);
    Ok((input, Frame::Comment { id: COMMENT_TAG.to_string(), size, flags, language: language.to_string(), description, value }))
  }
}

pub fn popularity_frame(len: fn(&[u8]) -> IResult<&[u8], u32>)
  -> impl FnMut(&[u8])
    -> IResult<&[u8], Frame> {
  move |input| {
    let (input, (_id, size, flags, email, rating)) =
      tuple((
        tag(POPULARITY_TAG),
        len,
        be_u16,
        terminated_utf8,
        be_u8,
      ))(input)?;
    let remaining = size - (email.len() + 2) as u32;
    let (input, _counter) = take(remaining)(input)?;
    Ok((input, Frame::Popularity { id: POPULARITY_TAG.to_string(), size, flags, email, rating }))
  }
}

pub fn object_frame(len: fn(&[u8]) -> IResult<&[u8], u32>)
  -> impl FnMut(&[u8])
    -> IResult<&[u8], Frame> {
  move |input| {
    let (input, (id, size, flags)) = tuple((tag(OBJECT_TAG), len, be_u16))(input)?;
    let id = from_utf8(id).unwrap().to_string();
    debug!("object {:?} {}",  id, size);
    let offset = input.len();
    let (input, encoding) = be_u8(input)?;
    let (input, mime_type) = terminated_utf8(input)?;
    let (input, (filename, description)) = encoded_string_pair(encoding, input)?;
    let remaining = size - (offset - input.len()) as u32;
    debug!("mime {}, filename {}, size {}, description {}", mime_type, filename, remaining, description);
    let (input, data) = take(remaining)(input)?;
    Ok((input, Frame::Object { id, size, flags, mime_type, filename, description, data: data.into() }))
  }
}

pub fn picture_frame(len: fn(&[u8]) -> IResult<&[u8], u32>)
  -> impl FnMut(&[u8])
    -> IResult<&[u8], Frame> {
  move |input| {
    let (input, (id, size, flags)) = tuple((tag(PICTURE_TAG), len, be_u16))(input)?;
    let id = from_utf8(id).unwrap().to_string();
    debug!("picture {:?} {}",  id, size);
    let start = input.len();
    let (input, encoding) = be_u8(input)?;
    let (input, mime_type) = terminated_utf8(input)?;
    let (input, kind) = be_u8(input)?;
    let (input, description) = encoded_string(encoding, input)?;
    let remaining = size - (start - input.len()) as u32;
    let (input, data) = take(remaining)(input)?;
    debug!("mime {}, size {}, description {}", mime_type, remaining, description);
    Ok((input, Frame::Picture { id, size, flags, mime_type, kind, description, data: data.into() }))
  }
}

pub fn text_frame(len: fn(&[u8]) -> IResult<&[u8], u32>)
  -> impl FnMut(&[u8])
    -> IResult<&[u8], Frame> {
  move |input| {
    let (input, (pid, id, size, flags)) =
      tuple((
        one_of("GT"),
        map(
          take(3u8),
          |res| from_utf8(res).unwrap(),
        ),
        len, be_u16))(input)?;
    let (input, (encoding, data)) = pair(be_u8, take(size - 1))(input)?;
    let (_data, text) = encoded_string(encoding, data)?;
    let merged = format!("{}{}", pid, id);
    debug!("utf8v23 {} {} {}", merged, size, text);
    Ok((input, Frame::Text { id: merged, size, flags, text }))
  }
}

pub fn generic_frame(len: fn(&[u8]) -> IResult<&[u8], u32>)
  -> impl FnMut(&[u8])
    -> IResult<&[u8], Frame> {
  move |input| {
    let (input, (id, size, flags)) =
      tuple((id_as_str, len, be_u16))(input)?;
    debug!("frame {} {}", id, size);
    let (input, data) = take(size)(input)?;
    Ok((input, Frame::Generic { id: id.to_string(), size, flags, data: data.into() }))
  }
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
    = tuple((tag("ID3"), be_u8, be_u8, be_u8, v24_len))(input)?;
  debug!("ID3 {} tag size {}", version, tag_size);
  Ok((input, Header { version, revision, flags, tag_size }))
}

pub fn as_syncsafe(total: u32) -> Vec<u8> {
  let mut result: Vec<u8> = Vec::new();
  let mut remaining = total;
  for _byte in total.to_be_bytes() {
    result.insert(0, (remaining & 0b01111111) as u8);
    remaining >>= 7;
  }
  result
}