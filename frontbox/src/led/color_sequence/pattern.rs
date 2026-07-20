use image::Rgba;

use crate::prelude::*;

pub fn render(pattern: &Vec<Rgba<u8>>, cycle: Cycle, length: usize) -> Vec<Rgba<u8>> {
  let mut pattern_offset: usize = 0;
  let mut cycle_count = 0;

  // foreach pixel in final result
  (0..length)
    .map(|_| {
      // check if the cycle has been exhausted or not
      let insert_pattern_pixel = match cycle {
        Cycle::Forever => true,
        Cycle::Once => cycle_count < 1,
        Cycle::Times(n) => cycle_count < n,
      };

      if insert_pattern_pixel {
        let pixel = pattern[pattern_offset].clone();

        pattern_offset += 1;
        if pattern_offset >= pattern.len() {
          cycle_count += 1;
          pattern_offset = 0;
        }

        pixel
      } else {
        Rgba::default()
      }
    })
    .collect()
}
