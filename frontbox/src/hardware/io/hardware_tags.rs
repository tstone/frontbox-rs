use dyn_clone::DynClone;
use std::any::Any;

pub trait HardwareTag: DynClone + Send + Sync + Any {
  fn as_any(&self) -> &dyn Any;
}

impl<T: Any + Send + Sync> HardwareTag for T
where
  T: Clone + Send + Sync,
{
  fn as_any(&self) -> &dyn Any {
    self
  }
}

dyn_clone::clone_trait_object!(HardwareTag);

pub mod tags {
  // switches
  #[derive(Clone)]
  pub struct Button;
  #[derive(Clone)]
  pub struct StartButton;
  #[derive(Clone)]
  pub struct ActionButton;
  #[derive(Clone)]
  pub struct FlipperButton;
  #[derive(Clone)]
  pub struct FlipperButtonLeft;
  #[derive(Clone)]
  pub struct FlipperButtonRight;
  #[derive(Clone)]
  pub struct LeftOutlane;
  #[derive(Clone)]
  pub struct LeftInlane;
  #[derive(Clone)]
  pub struct RightInlane;
  #[derive(Clone)]
  pub struct RightOutlane;
  #[derive(Clone)]
  pub struct AutoPlungerSwitch;
  #[derive(Clone)]
  pub struct CoinDoor;
  #[derive(Clone)]
  pub struct CoinDrop;
  #[derive(Clone)]
  pub struct Tilt;

  // drivers
  #[derive(Clone)]
  pub struct TroughCoil;
  #[derive(Clone)]
  pub struct AutoPlungerCoil;

  // multi
  #[derive(Clone)]
  pub struct Playfield;
  #[derive(Clone)]
  pub struct Cabinet;
  #[derive(Clone)]
  pub struct Lane;
  #[derive(Clone)]
  pub struct Ramp;
}
