use crate::*;

/// A type erased ``Positioned<Layer>``
pub trait PositionedLayer {
  fn render_relative(&self, parent: &mut CanvasView);
}

impl<L: Layer> PositionedLayer for Positioned<L> {
  /// Render layer relative to parent's position
  fn render_relative(&self, parent: &mut CanvasView) {
    let size = Size {
      width: self.placement.width.to_absolute(parent.bounds.width),
      height: self.placement.height.to_absolute(parent.bounds.height),
    };

    let origin_x = match self.placement.horizontal {
      Horizontal::Centered => (parent.bounds.width as i32 / 2) - (size.width as i32 / 2),
      Horizontal::LeftOffset(l) => l.to_absolute(parent.bounds.width as i32),
      Horizontal::RightOffset(r) => {
        (parent.bounds.width as i32 - size.width as i32) - r.to_absolute(parent.bounds.width as i32)
      }
    };

    let origin_y = match self.placement.vertical {
      Vertical::Centered => (parent.bounds.height as i32 / 2) - (size.height as i32 / 2),
      Vertical::TopOffset(t) => t.to_absolute(parent.bounds.height as i32),
      Vertical::BottomOffset(b) => {
        (parent.bounds.height as i32 - size.height as i32)
          - b.to_absolute(parent.bounds.height as i32)
      }
    };

    let origin = Position::new(origin_x, origin_y);
    let mut child = parent.child_view(origin, size);
    self.layer.render(&mut child);
  }
}
