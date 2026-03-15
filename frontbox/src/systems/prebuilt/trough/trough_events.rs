#[derive(Debug)]
#[allow(unused)]
pub struct BallEnteredTrough {
  pub occupancy: Vec<bool>,
}

impl BallEnteredTrough {
  pub fn new(occupancy: Vec<bool>) -> Box<BallEnteredTrough> {
    Box::new(Self { occupancy })
  }
}

#[derive(Debug)]
#[allow(unused)]
pub struct BallExitedTrough {
  pub occupancy: Vec<bool>,
}

impl BallExitedTrough {
  pub fn new(occupancy: Vec<bool>) -> Box<BallExitedTrough> {
    Box::new(Self { occupancy })
  }
}
