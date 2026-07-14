pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
  let r = r as f32 / 255.0;
  let g = g as f32 / 255.0;
  let b = b as f32 / 255.0;

  let max = r.max(g).max(b);
  let min = r.min(g).min(b);
  let delta = max - min;

  let h = if delta == 0.0 {
    0.0
  } else if max == r {
    60.0 * (((g - b) / delta).rem_euclid(6.0))
  } else if max == g {
    60.0 * (((b - r) / delta) + 2.0)
  } else {
    60.0 * (((r - g) / delta) + 4.0)
  };

  let s = if max == 0.0 { 0.0 } else { delta / max };
  let v = max;

  (h, s, v)
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
  let c = v * s;
  let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
  let m = v - c;

  let (r1, g1, b1) = match h as u32 {
    0..=59 => (c, x, 0.0),
    60..=119 => (x, c, 0.0),
    120..=179 => (0.0, c, x),
    180..=239 => (0.0, x, c),
    240..=299 => (x, 0.0, c),
    _ => (c, 0.0, x),
  };

  (
    ((r1 + m) * 255.0).round() as u8,
    ((g1 + m) * 255.0).round() as u8,
    ((b1 + m) * 255.0).round() as u8,
  )
}
