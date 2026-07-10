use dyn_clone::DynClone;
use std::any::Any;
use std::fmt::Debug;

pub trait Tag: Debug + DynClone + Send + Sync + Any {
  fn as_any(&self) -> &dyn Any;
}

impl<T: Any + Send + Sync> Tag for T
where
  T: Debug + Clone + Send + Sync,
{
  fn as_any(&self) -> &dyn Any {
    self
  }
}

dyn_clone::clone_trait_object!(Tag);

// switches
#[derive(Clone, Debug)]
pub struct Button;
#[derive(Clone, Debug)]
pub struct StartButton;
#[derive(Clone, Debug)]
pub struct ActionButton;
#[derive(Clone, Debug)]
pub struct FlipperButton;
#[derive(Clone, Debug)]
pub struct FlipperButtonLeft;
#[derive(Clone, Debug)]
pub struct FlipperButtonRight;
#[derive(Clone, Debug)]
pub struct LeftOutlane;
#[derive(Clone, Debug)]
pub struct LeftInlane;
#[derive(Clone, Debug)]
pub struct RightInlane;
#[derive(Clone, Debug)]
pub struct RightOutlane;
#[derive(Clone, Debug)]
pub struct AutoPlungerSwitch;
#[derive(Clone, Debug)]
pub struct CoinDoor;
#[derive(Clone, Debug)]
pub struct CoinDrop;
#[derive(Clone, Debug)]
pub struct Tilt;
#[derive(Clone, Debug)]
pub struct SlingShot;

// drivers
#[derive(Clone, Debug)]
pub struct TroughCoil;
#[derive(Clone, Debug)]
pub struct AutoPlungerCoil;

// multi
#[derive(Clone, Debug)]
pub struct Playfield;
#[derive(Clone, Debug)]
pub struct Cabinet;
#[derive(Clone, Debug)]
pub struct Lane;
#[derive(Clone, Debug)]
pub struct Ramp;
