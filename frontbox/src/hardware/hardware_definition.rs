/*!
 * ## Hardware Definition
 *
 * Hardware moves in to 3 states:
 *  1. Definition - User defined attributes, including configuration (device-specific)
 *  2. Wired - A definition assigned to a specific board and pin
 *  3. Addressed - A resolved ID on the network
 */

use std::borrow::Cow;

use glam::prelude::Vec3;

use crate::prelude::*;

/// Core of all hardware attributes
pub trait HardwareDefinition {
  fn name(&self) -> Cow<'static, str>;
  fn tags(&self) -> Vec<Box<dyn Tag>>;
  /// Location in 3D space, relative to the bottom left corner of the cabinet
  /// Use the playfield reference plane for easier
  fn location(&self) -> Option<Vec3>;

  /// Query by name
  fn q(&self) -> HardwareQuery {
    HardwareQuery::Name(self.name().to_string())
  }
}

// -- IO --

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IoAddress {
  pub board_idx: u8,
  pub pin: u16,
}

impl IoAddress {
  pub fn new(board_idx: u8, pin: u16) -> Self {
    Self { board_idx, pin }
  }
}

#[derive(Debug, Clone)]
pub struct IoWired<D: HardwareDefinition + Clone> {
  pub definition: D,
  pub assignment: IoAddress,
}

impl<T> IoWired<T>
where
  T: HardwareDefinition + Clone,
{
  pub fn new(definition: T, assignment: IoAddress) -> Self {
    IoWired {
      definition,
      assignment,
    }
  }
}

pub struct IoAddressed<T: HardwareDefinition> {
  pub definition: T,
  pub assignment: IoAddress,
  pub id: usize,
}

// -- Exp --

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExpAddress {
  pub board_address: u8,
  pub breakout: Option<u8>,
  pub port: u8,
}

impl ExpAddress {
  pub fn new(board_address: u8, breakout: Option<u8>, port: u8) -> Self {
    Self {
      board_address,
      breakout,
      port,
    }
  }
}

#[derive(Debug, Clone)]
pub struct ExpWired<D: HardwareDefinition + Clone> {
  pub definition: D,
  pub assignment: ExpAddress,
}

impl<T> ExpWired<T>
where
  T: HardwareDefinition + Clone,
{
  pub fn new(definition: T, assignment: ExpAddress) -> Self {
    ExpWired {
      definition,
      assignment,
    }
  }
}

#[derive(Debug, Clone)]
pub struct ExpAddressed<T: HardwareDefinition> {
  pub definition: T,
  pub assignment: ExpAddress,
  pub id: usize,
}
