use std::io::{self, Cursor};
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Decodes the sound in `bytes`, from the file `name`, into the mean square of its channels at each frame, and how many
/// frames a second it has. That's all an analysis of its loudness needs, whatever its channels. Errors say why it
/// couldn't be decoded.
pub(crate) fn decode_power(name: &str, bytes: Vec<u8>) -> Result<(Vec<f32>, u32), String> {
  let mut hint = Hint::new();
  if let Some(extension) = Path::new(name).extension().and_then(|e| e.to_str()) {
    hint.with_extension(extension);
  }
  let stream = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
  let probed = symphonia::default::get_probe()
    .format(
      &hint,
      stream,
      &FormatOptions::default(),
      &MetadataOptions::default(),
    )
    .map_err(|error| error.to_string())?;
  let mut format = probed.format;
  let track = format
    .default_track()
    .ok_or_else(|| "it has no audio".to_owned())?;
  let track_id = track.id;
  let mut decoder = symphonia::default::get_codecs()
    .make(&track.codec_params, &DecoderOptions::default())
    .map_err(|error| error.to_string())?;

  let mut power = Vec::new();
  let mut sample_rate = track.codec_params.sample_rate;
  loop {
    let packet = match format.next_packet() {
      Ok(packet) => packet,
      Err(Error::IoError(error)) if error.kind() == io::ErrorKind::UnexpectedEof => break,
      Err(error) => return Err(error.to_string()),
    };
    if packet.track_id() != track_id {
      continue;
    }
    let decoded = match decoder.decode(&packet) {
      Ok(decoded) => decoded,
      // a damaged packet is skipped, as a player would
      Err(Error::DecodeError(_)) => continue,
      Err(error) => return Err(error.to_string()),
    };
    let spec = *decoded.spec();
    sample_rate.get_or_insert(spec.rate);
    let channels = spec.channels.count().max(1);
    let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
    samples.copy_interleaved_ref(decoded);
    power.extend(
      samples
        .samples()
        .chunks(channels)
        .map(|frame| frame.iter().map(|sample| sample * sample).sum::<f32>() / channels as f32),
    );
  }
  match sample_rate {
    Some(rate) if rate > 0 => Ok((power, rate)),
    _ => Err("its sample rate isn't known".into()),
  }
}

#[cfg(test)]
pub(crate) mod tests {
  use super::*;

  /// A 16-bit mono WAV file of `samples` at `rate` frames a second
  pub(crate) fn wav(rate: u32, samples: &[i16]) -> Vec<u8> {
    let data = (samples.len() * 2) as u32;
    let mut bytes = Vec::new();
    bytes.extend(b"RIFF");
    bytes.extend((36 + data).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes()); // PCM
    bytes.extend(1u16.to_le_bytes()); // mono
    bytes.extend(rate.to_le_bytes());
    bytes.extend((rate * 2).to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(data.to_le_bytes());
    samples
      .iter()
      .for_each(|sample| bytes.extend(sample.to_le_bytes()));
    bytes
  }

  #[test]
  fn it_decodes_a_wav_into_each_frames_power() {
    let half = i16::MAX / 2;
    let (power, rate) = decode_power("a.wav", wav(8000, &[0, half, -half, 0])).unwrap();
    assert_eq!(rate, 8000);
    let rounded: Vec<f32> = power.iter().map(|p| (p * 100.0).round() / 100.0).collect();
    assert_eq!(rounded, [0.0, 0.25, 0.25, 0.0]);
  }

  #[test]
  fn what_isnt_a_sound_is_an_error() {
    assert!(decode_power("a.wav", b"not a sound".to_vec()).is_err());
  }
}
