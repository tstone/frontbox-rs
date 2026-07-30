use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, Frame};

pub struct Gif;

impl Gif {
  pub fn decode_from_path(path: impl AsRef<Path>) -> Vec<Frame> {
    let path = path.as_ref();
    let file_in =
      BufReader::new(File::open(path).unwrap_or_else(|_| panic!("Failed to load gif at {:?}", path)));
    let decoder = GifDecoder::new(file_in).unwrap();
    let frames = decoder.into_frames();
    frames.collect_frames().expect("error decoding gif")
  }
}
