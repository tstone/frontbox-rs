use image::Rgba;

use crate::prelude::*;

pub fn render(pattern: &Vec<Rgba<u8>>, cycle: Cycle, length: u16) -> Vec<Rgba<u8>> {
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn once() {
    let colors = render(&vec![Rgba::red(), Rgba::white()], Cycle::Once, 3);

    assert_eq!(colors.len(), 3);
    assert_eq!(colors[0], Rgba::red());
    assert_eq!(colors[1], Rgba::white());
    assert_eq!(colors[2], Rgba::default());
  }

    #[test]
  fn once_under() {
    let colors = render(&vec![Rgba::red(), Rgba::white(), Rgba::blue()], Cycle::Once, 1);

    assert_eq!(colors.len(), 1);
    assert_eq!(colors[0], Rgba::red());
  }

  #[test]
  fn twice() {
    let colors = render(&vec![Rgba::red(), Rgba::white()], Cycle::Times(2), 5);

    assert_eq!(colors.len(), 5);
    assert_eq!(colors[0], Rgba::red());
    assert_eq!(colors[1], Rgba::white());
    assert_eq!(colors[2], Rgba::red());
    assert_eq!(colors[3], Rgba::white());
    assert_eq!(colors[4], Rgba::default());
  }

  #[test]
  fn forever() {
    let colors = render(&vec![Rgba::red()], Cycle::Forever, 3);

    assert_eq!(colors.len(), 3);
    assert_eq!(colors[0], Rgba::red());
    assert_eq!(colors[1], Rgba::red());
    assert_eq!(colors[2], Rgba::red());
  }
}