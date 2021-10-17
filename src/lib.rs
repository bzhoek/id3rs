use nom::bytes::streaming::tag;
use nom::IResult;
use nom::multi::fold_many_m_n;
use nom::number::streaming::be_u8;
use nom::sequence::tuple;

#[derive(Debug, PartialEq, Eq)]
pub struct Header {
  revision: u8,
  flags: u8,
  next: u64,
}

fn file_header(input: &[u8]) -> IResult<&[u8], Header> {
  let (input, (_, revision, flags, next)) = tuple(
    (tag("ID3"),
      be_u8,
      be_u8,
      fold_many_m_n(4, 4, be_u8, 0u64, |acc, byte| acc << 7 | (byte as u64))
    ))(input)?;
  Ok((input, Header { revision, flags, next }))
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
    assert_eq!(header, Header { revision: 3, flags: 0, next: 359 });
  }
}