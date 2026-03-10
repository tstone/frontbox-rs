use frontbox::prelude::*;
use std::io::Write;

#[tokio::main]
async fn main() {
  env_logger::Builder::from_default_env()
    .format(|buf, record| writeln!(buf, "[{}] {}\r", record.level(), record.args()))
    .init();

  MachineBuilder::boot(BootConfig::default(), IoNetworkSpec::new().build(), vec![])
    .await
    .build()
    .run(vec![ExampleSystem::new()])
    .await;
}

#[derive(Default, Serialize, Storable, PartialEq)]
enum ExampleStateMachine {
  #[default]
  State1,
  State2,
  State3,
}

struct ExampleSystem;

impl ExampleSystem {
  pub fn new() -> Box<Self> {
    Box::new(Self)
  }
}

impl System for ExampleSystem {
  fn on_startup(&mut self, _ctx: &Context, cmds: &mut Commands) {
    // transition into a state
    cmds.transition(ExampleStateMachine::State2);
  }

  fn is_active(&self, ctx: &Context) -> bool {
    // require a specific state to be active for this system to run (receive events, timer ticks, etc.)
    ctx.states.is(ExampleStateMachine::State2)
  }
}
