#[cfg(test)]
mod tests {
  use id3rs::frame::{frame_header, FrameHeader, Layer, Protection, Version};

  #[test]
  fn find_frame_header() {
    let buffer = include_bytes!("../samples/4tink.mp3");
    let (position, frame) = frame_header(&buffer[1114..]).ok().unwrap();
    assert_eq!(buffer.len() - position.len(), 1128);
    println!("{:?}", frame);
    assert_eq!(frame, FrameHeader {
      version: Version::Version1,
      layer: Layer::Layer3,
      crc: Protection::Unprotected,
      bitrate: 160,
    });
  }
}