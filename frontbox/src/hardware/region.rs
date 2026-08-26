use glam::Vec2;
use num_traits::Pow;

#[derive(Debug, Clone, PartialEq)]
pub enum Region {
  Circle { center: Vec2, radius: f32 },
  Rect { top_left: Vec2, bottom_right: Vec2 }
}

impl Region {
  pub fn within(&self, point: Vec2) -> bool {
    match self {
      Self::Rect { top_left, bottom_right } => {
        point.x >= top_left.x && point.y >= top_left.y && point.x <= bottom_right.x && point.y <= bottom_right.y
      }
      Self::Circle { center, radius } => {
        ((point.x - center.x).pow(2) + (point.y - center.y).pow(2)) < radius.pow(2)
      }
    }
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rect_within_true() {
    let region = Region::Rect { top_left: Vec2::new(0.0, 0.0), bottom_right: Vec2::new(10.0, 10.0) };

    assert_eq!(region.within(Vec2::new(0.0, 0.0)), true);
    assert_eq!(region.within(Vec2::new(0.0, 10.0)), true);
    assert_eq!(region.within(Vec2::new(10.0, 0.0)), true);
    assert_eq!(region.within(Vec2::new(5.0, 5.0)), true);    
    assert_eq!(region.within(Vec2::new(10.0, 10.0)), true);
  }

  #[test]
  fn rect_within_false() {
    let region = Region::Rect { top_left: Vec2::new(0.0, 0.0), bottom_right: Vec2::new(10.0, 10.0) };

    assert_eq!(region.within(Vec2::new(-1.0, -1.0)), false);
    assert_eq!(region.within(Vec2::new(10.1, 10.1)), false);
  }

  #[test]
  fn circle_within_true() {
    let region = Region::Circle { center: Vec2::new(0.0, 0.0), radius: 5.0 };

    assert_eq!(region.within(Vec2::new(0.0, 0.0)), true);
    assert_eq!(region.within(Vec2::new(2.5, 2.5)), true);
  }

  #[test]
  fn circle_within_false() {
    let region = Region::Circle { center: Vec2::new(0.0, 0.0), radius: 5.0 };

    assert_eq!(region.within(Vec2::new(-5.0, -5.0)), false);
    assert_eq!(region.within(Vec2::new(10.1, 10.1)), false);
  }
}
