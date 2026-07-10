use crate::prelude::Tag;

/**
 * Hardware moves in to 3 states:
 *  1. Definition - User defined attributes, including configuration (device-specific)
 *  2. Wired - A definition assigned to a specific board and pin
 *  3. Addressed - A resolved ID on the network
 */
pub trait HardwareDefinition: 'static {
  fn name(&self) -> &'static str;
  fn tags(&self) -> Vec<Box<dyn Tag>>;
  fn locations(&self) -> Vec<Location>;

  fn get_location(&self, tag: &dyn Tag) -> Option<Location> {
    let type_id = tag.as_any().type_id();
    self
      .locations()
      .into_iter()
      .find(|loc| <dyn Tag>::as_any(loc.tag.as_ref()).type_id() == type_id)
  }
}

/// Physical location of a piece of hardware, identified by a tag
/// Hardware can have multiple relative locations, e.g. playfield or backbox or sub-region
#[derive(Debug, Clone)]
pub struct Location {
  pub tag: Box<dyn Tag>,
  pub x: f32,
  pub y: f32,
}

impl Location {
  pub fn new(tag: Box<dyn Tag>, x: f32, y: f32) -> Self {
    Self { tag, x, y }
  }
}

#[derive(Debug, Clone)]
pub enum BoardAssignment {
  IO {
    board_idx: u8,
    pin: u16,
  },
  Exp {
    board_address: u8,
    breakout: Option<u8>,
    port: u8,
  },
}

#[derive(Debug, Clone)]
pub struct Wired<T: HardwareDefinition + Clone> {
  pub definition: T,
  pub assignment: BoardAssignment,
}

impl<T> Wired<T>
where
  T: HardwareDefinition + Clone,
{
  pub fn new(definition: T, assignment: BoardAssignment) -> Self {
    Wired {
      definition,
      assignment,
    }
  }
}

pub struct Addressed<T: HardwareDefinition> {
  pub definition: T,
  pub assignment: BoardAssignment,
  pub id: usize,
}
