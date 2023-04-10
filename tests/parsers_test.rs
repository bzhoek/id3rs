#[cfg(test)]
mod tests {
  use std::io::Read;

  use assert_matches::assert_matches;

  use id3rs::*;
  use id3rs::parsers::{all_frames_v24, comment_frame_v24, extended_text_frame_v24, file_header, generic_frame_v24, object_frame_v24, padding, text_frame_v24};

  #[test]
  fn test_header_and_frames() {
    let (rofile, _, _) = filenames("samples/4tink");
    let mut file = std::fs::File::open(&rofile).unwrap();
    let mut buffer = [0; 10];
    file.read_exact(&mut buffer).unwrap();

    let (_, header) = file_header(&buffer).ok().unwrap();
    assert_eq!(header, Header { version: 4, revision: 0, flags: 0, tag_size: 1114 });

    let mut input = vec![0u8; header.tag_size as usize];
    file.read_exact(&mut input).unwrap();

    let (_, result) = all_frames_v24(&input).ok().unwrap();
    assert_eq!(12, result.len());
  }

  #[test]
  fn test_frames_individually() {
    log_init();

    let (rofile, _, _) = filenames("samples/4tink");
    let mut file = std::fs::File::open(&rofile).unwrap();
    let mut buffer = [0; 10];
    file.read_exact(&mut buffer).unwrap();

    let (_, header) = file_header(&buffer).ok().unwrap();
    assert_eq!(header, Header { version: 4, revision: 0, flags: 0, tag_size: 1114 });

    let mut input = vec![0u8; header.tag_size as usize];
    file.read_exact(&mut input).unwrap();

    let data = "Hello, world".as_bytes().to_vec();
    let (input, frame) = object_frame_v24(&input).ok().unwrap();
    assert_eq!(frame, Frame::Object { id: OBJECT_TAG.to_string(), size: 80, flags: 0, mime_type: "application/vnd.rekordbox.dat".to_string(), filename: "ANLZ0000.DAT".to_string(), description: "Rekordbox Analysis Data".to_string(), data });

    let (input, frame) = extended_text_frame_v24(&input).ok().unwrap();
    assert_eq!(frame, Frame::ExtendedText { id: EXTENDED_TAG.to_string(), size: 12, flags: 0, description: "Hello".to_string(), value: "World".to_string() });

    let (input, frame) = text_frame_v24(&input).ok().unwrap();
    assert_eq!(frame, Frame::Text { id: TITLE_TAG.to_string(), size: 5, flags: 0, text: "Tink".to_string() });

    let (input, frame) = text_frame_v24(&input).ok().unwrap();
    assert_eq!(frame, Frame::Text { id: ARTIST_TAG.to_string(), size: 6, flags: 0, text: "Apple".to_string() });

    let (input, frame) = comment_frame_v24(&input).ok().unwrap();
    assert_matches!(frame, Frame::Comment{ id, value, ..} => {
      assert_eq!(id, COMMENT_TAG);
      assert_eq!(value, "From Big Sur");
    });

    let (input, frame) = text_frame_v24(&input).ok().unwrap();
    assert_eq!(frame, Frame::Text { id: GENRE_TAG.to_string(), size: 7, flags: 0, text: "sounds".to_string() });

    let (input, frame) = extended_text_frame_v24(&input).ok().unwrap();
    assert_eq!(frame, Frame::ExtendedText { id: EXTENDED_TAG.to_string(), size: 23, flags: 0, description: "こんにちは".to_string(), value: "世界".to_string() });

    let (input, frame) = text_frame_v24(&input).ok().unwrap();
    assert_eq!(frame, Frame::Text { id: KEY_TAG.to_string(), size: 3, flags: 0, text: "4A".to_string() });

    let (input, frame) = extended_text_frame_v24(&input).ok().unwrap();
    assert_eq!(frame, Frame::ExtendedText { id: EXTENDED_TAG.to_string(), size: 14, flags: 0, description: "EnergyLevel".to_string(), value: "6".to_string() });

    let (input, frame) = text_frame_v24(&input).ok().unwrap();
    assert_eq!(frame, Frame::Text { id: SUBTITLE_TAG.to_string(), size: 1, flags: 0, text: "".to_string() });

    let (input, frame) = generic_frame_v24(&input).ok().unwrap();
    assert_matches!(frame, Frame::Generic{ id, ..} => {
      assert_eq!(id, GROUPING_TAG);
    });

    let (_input, frame) = padding(&input).ok().unwrap();
    assert_eq!(frame, Frame::Padding { size: 831 });
  }

  fn filenames(base: &str) -> (String, String, String) {
    (format!("{}.mp3", base), format!("{}-out.mp3", base), format!("{}-rw.mp3", base))
  }
}