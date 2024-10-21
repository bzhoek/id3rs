pub const ID3FRAME_SIZE: u32 = 10;

#[cfg(test)]
mod tests {
  use std::convert::TryInto;
  use std::fs;

  use assert_matches::assert_matches;

  use id3rs::{Frame, GENRE_TAG, ID3rs, log_init, make_rwcopy, mpck};
  use id3rs::parsers::as_syncsafe;
  use crate::ID3FRAME_SIZE;

  mod v23 {
    use std::process::exit;
    use std::str::from_utf8;

    use id3rs::{EXTENDED_TAG, Frame, ID3rs, log_init, OBJECT_TAG, TITLE_TAG};

    use super::*;

    const FILENAME: &str = "samples/3tink";

    #[test]
    pub fn test_adding() {
      log_init();
      let file = "/Users/bas/Downloads/1. Nova -- Tale Of Us [508821602]".to_string();
      rw_test(&file, |(_, _, rwfile)| {
        let mut tag = ID3rs::read(&rwfile).unwrap();
        tag.set_title("Hello");
        tag.set_artist("World");
        tag.write_to(rwfile).unwrap();
        exit(0);
      });
    }

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
    pub fn test_all_objects() {
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
    pub fn test_cue_points() {
      log_init();
      let tag = ID3rs::read("/Users/bas/bzhoek/happer/tests/0. Psycho Killer -- Talking Heads [5093662].mp3").unwrap();
      let data = tag.object_by_description("CuePoints").and_then(|f| match f {
        Frame::Object { data, .. } => Some(data),
        _ => None
      });


      let result = data.map(|data| from_utf8(&data).ok()).flatten()
        .map(|str| str.replace('\n', ""));
      assert_eq!(result, Some("eyJhbGdvcml0aG0iOjE0LCJjdWVzIjpbeyJuYW1lIjoiQ3VlIDEiLCJ0aW1lIjo0LjQyNDg1NzU4NTM0MTYxNDZ9LHsibmFtZSI6IkN1ZSAyIiwidGltZSI6MTU1MzguODk5OTc5OTQyNzc3fSx7Im5hbWUiOiJDdWUgMyIsInRpbWUiOjQ2NjA3Ljg1MDIyNDY1NzY0Nn0seyJuYW1lIjoiQ3VlIDQiLCJ0aW1lIjo2MjE0Mi4zMjUzNDcwMTUwODF9LHsibmFtZSI6IkN1ZSA1IiwidGltZSI6MTU1MzQ5LjE3NjA4MTE1ODY5fSx7Im5hbWUiOiJDdWUgNiIsInRpbWUiOjE3MDg4My42NTEyMDM1MTU5NH0seyJuYW1lIjoiQ3VlIDciLCJ0aW1lIjoyMDE5NTIuNjAxNDQ4MjMwNTN9LHsibmFtZSI6IkN1ZSA4IiwidGltZSI6MjE3NDg3LjA3NjU3MDU4Nzc5fV0sInNvdXJjZSI6Im1peGVkaW5rZXkifQ==".to_string()))
    }

    #[test]
    pub fn test_find_object() {
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
    use id3rs::{mpck, Picture, PICTURE_TAG, TITLE_TAG};

    use super::*;

    const FILENAME: &str = "samples/4tink";

    #[test]
    pub fn test_set_object() {
      rw_test(FILENAME, |(_, _, rwfile)| {
        let mut tag = ID3rs::read(&rwfile).unwrap();

        tag.set_object("HELLO.TXT", "text/plain", "Hello", &"Hello, world".as_bytes());
        tag.set_extended_text("EnergyLevel", "99");
        tag.write_to(&rwfile).unwrap();
      });
    }

    #[test]
    pub fn test_change_extended_text() {
      rw_test(FILENAME, |(rofile, _, rwfile)| {
        let mut tag = ID3rs::read(&rwfile).unwrap();
        tag.set_extended_text("OriginalTitle", &tag.title().unwrap().to_string());
        tag.set_extended_text("EnergyLevel", "99");
        tag.write_to(&rwfile).unwrap();

        let tag = ID3rs::read(&rwfile).unwrap();
        assert_eq!(tag.extended_text("OriginalTitle"), Some("Tink"));
        assert_eq!(tag.extended_text("EnergyLevel"), Some("99"));
        assert_eq!(mpck(&rofile), mpck(&rwfile));
      });
    }

    #[test]
    pub fn test_set_popularity() {
      rw_test(FILENAME, |(rofile, _, rwfile)| {
        let mut tag = ID3rs::read(&rwfile).unwrap();
        tag.set_popularity("traktor@native-instruments.de", 3);
        tag.set_track(1,1);
        tag.write_to(&rwfile).unwrap();

        let tag = ID3rs::read(&rwfile).unwrap();
        assert_eq!(tag.track(), Some("1/1"));
        assert_eq!(mpck(&rofile), mpck(&rwfile));
      });
    }

    #[test]
    pub fn test_set_track() {
      rw_test(FILENAME, |(rofile, _, rwfile)| {
        let mut tag = ID3rs::read(&rwfile).unwrap();
        tag.set_track(1,1);
        tag.write_to(&rwfile).unwrap();

        let tag = ID3rs::read(&rwfile).unwrap();
        assert_eq!(tag.track(), Some("1/1"));
        assert_eq!(mpck(&rofile), mpck(&rwfile));
      });
    }

    #[test]
    pub fn test_attach_picture() {
      rw_test(FILENAME, |(_, _, rwfile)| {
        let mut tag = ID3rs::read(&rwfile).unwrap();
        let cover = fs::read("samples/cover.jpg").unwrap();
        tag.set_attached_picture(Picture::FrontCover, "image/jpg", "cover", &*cover);
        tag.write_to(&rwfile).unwrap();

        let tag = ID3rs::read(&rwfile).unwrap();
        let picture = tag.attached_picture(Picture::FrontCover).unwrap();
        assert_matches!(picture, Frame::Picture { data, .. } => {
          assert_eq!(cover.len(), data.len());
        });
      });
    }

    #[test]
    pub fn test_attached_picture() {
      log_init();
      let tag = ID3rs::read("samples/3tank.mp3").unwrap();
      let bzhoek = fs::read("samples/bzhoek.png").unwrap();
      let picture = tag.attached_picture(Picture::FrontCover).unwrap();
      assert_matches!(picture, Frame::Picture { id, data, mime_type, .. } => {
        assert_eq!(id, PICTURE_TAG);
        assert_eq!(mime_type, "image/png");
        assert_eq!(bzhoek.len(), data.len());
        assert_eq!(&bzhoek, data);
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
      tag.write_to(&rwfile).unwrap();

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
      tag.write_to(&outfile).unwrap();
      assert_eq!(mpck(&rofile), mpck(&outfile));
    });
  }

  #[test]
  pub fn test_change_copy() {
    rw_test("samples/4tink", |(rofile, outfile, _)| {
      let mut tag = ID3rs::read(&rofile).unwrap();
      tag.set_title("Bleek");
      tag.set_extended_text("EnergyLevel", "99");
      tag.write_to(&outfile).unwrap();
      assert_eq!(mpck(&rofile), mpck(&outfile));
    });
  }

  #[test]
  pub fn test_change_inplace() {
    rw_test("samples/4tink", |(rofile, _, rwfile)| {
      let mut tag = ID3rs::read(&rwfile).unwrap();
      tag.set_title("Bleek");
      tag.set_extended_text("EnergyLevel", "99");
      tag.write_to(&rwfile).unwrap();
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
        Frame::Generic { size, .. } => ID3FRAME_SIZE + size,
        Frame::Text { size, .. } => ID3FRAME_SIZE + size,
        Frame::Comment { size, .. } => ID3FRAME_SIZE + size,
        Frame::ExtendedText { size, .. } => ID3FRAME_SIZE + size,
        Frame::Object { size, .. } => ID3FRAME_SIZE + size,
        Frame::Padding { size } => 0 + size,
        Frame::Picture { size, .. } => ID3FRAME_SIZE + size,
        Frame::Popularity { .. } => 0
      });

    assert_eq!(sum, 1114);

    let _sum = tag.frames.iter()
      .fold(0u32, |sum, frame| sum + match frame {
        Frame::Generic { size, .. } => ID3FRAME_SIZE + size,
        Frame::Text { text, .. } => ID3FRAME_SIZE + 1 + text.len() as u32,
        Frame::Comment { size, .. } => ID3FRAME_SIZE + size,
        Frame::ExtendedText { size, .. } => ID3FRAME_SIZE + size,
        Frame::Object { size, .. } => ID3FRAME_SIZE + size,
        Frame::Padding { size } => 0 + size,
        Frame::Picture { size, .. } => ID3FRAME_SIZE + size,
        Frame::Popularity { .. } => 0
      });

    let _double_utf16 = 15 + 23 + 11 + 3 + 15 + (5 * 2); // 67
  }

  fn filenames(base: &str) -> (String, String, String) {
    let rnd = rand::random::<u32>();
    (format!("{}.mp3", base), format!("{}-out{}.mp3", base, rnd), format!("{}-rw{}.mp3", base, rnd))
  }
}