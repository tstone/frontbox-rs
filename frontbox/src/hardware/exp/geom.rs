#[derive(Debug, Clone)]
pub struct Point(pub f32, pub f32);

#[derive(Debug, Clone)]
pub enum RenderableGeom {
  Circle {
    center: Point,
    radius: f32,
  },
  Rectangle {
    top_left: Point,
    bottom_right: Point,
  },
}

#[derive(Debug, Clone)]
pub struct Bitmap {
  pub height: u16,
  pub width: u16,
  /// Color channels arranged as r,g,b,a
  pub data: Vec<f32>,
}
