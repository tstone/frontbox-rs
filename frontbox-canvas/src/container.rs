use frontbox::prelude::*;

use crate::*;

pub struct Container {
  pub padding: Padding,
  rectangle: Rectangle,
  pub layers: Vec<Box<dyn PositionedLayer>>,
}

impl Default for Container {
  fn default() -> Self {
    Self {
      padding: Padding::zero(),
      rectangle: Rectangle::new(Fill2d::Transparent),
      layers: Vec::new(),
    }
  }
}

impl Container {
  pub fn new(fill: Fill2d) -> Self {
    Self {
      rectangle: Rectangle::new(fill),
      ..Default::default()
    }
  }

  pub fn transparent() -> Self {
    Self::new(Fill2d::Transparent)
  }

  pub fn with_border(mut self, width: u8, color: Rgba<u8>) -> Self {
    self.rectangle.border = Some(Border::new(width, color));
    self
  }

  pub fn with_padding(mut self, left: u32, top: u32, right: u32, bottom: u32) -> Self {
    self.padding = Padding::new(left, top, right, bottom);
    self
  }

  /// Set all padding values to the same value
  pub fn with_padding_all(mut self, v: u32) -> Self {
    self.padding = Padding::new(v, v, v, v);
    self
  }

  pub fn border_mut(&mut self) -> &mut Option<Border> {
    &mut self.rectangle.border
  }

  pub fn fill_mut(&mut self) -> &mut Fill2d {
    &mut self.rectangle.fill
  }

  /// Add a layer above all other layers
  pub fn add(&mut self, layer: impl PositionedLayer + 'static) {
    self.layers.push(Box::new(layer));
  }
}

impl Layer for Container {
  fn render<'a>(&self, canvas: &mut CanvasView<'a>) {
    self.rectangle.render(canvas);

    // remove border and padding from viewport
    let border_width = match &self.rectangle.border {
      Some(b) => b.width as u32,
      None => 0,
    };

    // actual renderable area minus padding and border
    let usable_bounds = Size {
      width: canvas.bounds.width - self.padding.left - self.padding.right - (border_width * 2),
      height: canvas.bounds.height - self.padding.top - self.padding.bottom - (border_width * 2),
    };
    let mut child_canvas = canvas.child_view(
      Position::new(
        (self.padding.left + border_width) as i32,
        (self.padding.top + border_width) as i32,
      ),
      usable_bounds,
    );

    for layer in &self.layers {
      layer.render_relative(&mut child_canvas);
    }
  }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Padding {
  left: u32,
  right: u32,
  top: u32,
  bottom: u32,
}

impl Padding {
  pub fn zero() -> Self {
    Padding::default()
  }

  pub fn new(left: u32, top: u32, right: u32, bottom: u32) -> Self {
    Self {
      left,
      right,
      top,
      bottom,
    }
  }
}
