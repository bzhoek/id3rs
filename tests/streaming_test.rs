use log::LevelFilter;

#[cfg(test)]
#[ctor::ctor]
fn init() {
  let _ = env_logger::builder().is_test(true).filter_level(LevelFilter::Debug).try_init();
}

#[cfg(test)]
mod tests {
  use id3rs::mp3_frame::FrameHeader;
  use id3rs::mp3_parser::Mp3FrameParser;
  use std::fs::File;
  use std::io::Write;

  #[test]
  fn test_writing() {
    let mut file = File::create("frames.mp3").unwrap();
    let file_iter = Mp3FrameParser::new("samples/4tink.mp3").unwrap();
    for header in file_iter {
      file.write_all(&*header.data).unwrap();
    }
    file.flush().unwrap();
  }

  #[test]
  fn test_iterator() {
    let file_iter = Mp3FrameParser::new("samples/4tink.mp3").unwrap();
    let frames: Vec<_> = file_iter.collect();
    assert_eq!(26, frames.len());
  }

  #[test]
  fn test_layer3_size() {
    let header = FrameHeader {
      version: id3rs::mp3_frame::Version::Version1,
      layer: id3rs::mp3_frame::Layer::Layer3,
      crc: id3rs::mp3_frame::Protection::Unprotected,
      bitrate: 128,
      frequency: 44100,
      padding: 0,
      data: vec![],
    };
    let size = header.frame_size();
    assert_eq!(417, size);
  }
}
