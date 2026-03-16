use fast_protocol::EventResponse;
use tokio::sync::mpsc;

use crate::machine::event_interrupt_registry::EventInterruptRegistry;
use crate::prelude::*;
use crate::systems::SystemCommandsProcessor;
use crate::systems::SystemContainer;
use crate::systems::SystemMessage;
use crate::systems::run_system_timers;

pub async fn run(
  mut machine: Machine,
  mut store: Store,
  config: AppConfig,
  mut initial_systems: Vec<Box<dyn System>>,
) {
  let mut interrupt_registry = EventInterruptRegistry::new();
  let mut command_registry = CommandRegistry::new();

  let (app_sender, mut app_receiver) = mpsc::unbounded_channel::<AppMessage>();
  let (system_sender, mut system_receiver) = mpsc::unbounded_channel::<SystemMessage>();
  machine.set_app_sender(app_sender.clone());

  // initialize systems
  let mut systems: Vec<SystemContainer> = vec![];
  initial_systems.push(MachineBridge::new(machine.machine_sender()));

  for system in initial_systems {
    SystemCommandsProcessor::spawn_system(
      system,
      &mut systems,
      &mut store,
      app_sender.clone(),
      system_sender.clone(),
    );
  }
  // TODO: review how this works and make sure it still makes sense in the new architecture
  run_system_timers(config.system_timer_tick.clone(), app_sender.clone());

  // listen for ctrl-c to trigger shutdown
  let tx = app_sender.clone();
  tokio::spawn(async move {
    tokio::signal::ctrl_c()
      .await
      .expect("failed to listen for ctrl-c");
    let _ = tx.send(AppMessage::Shutdown);
  });

  log::info!("⟳ Run loop started.");
  loop {
    tokio::select! {
      Some(hardware_event) = machine.read_io() => {
        match hardware_event {
          EventResponse::Switch { switch_id, state } => {
            let mut ctx = Context::new(&mut store, 0, app_sender.clone(), system_sender.clone());
            machine.handle_switch_event(switch_id, state, &mut ctx);
          }
        }
      }

      Some(command) = app_receiver.recv() => {
        match command {
          AppMessage::EmitEvent(event) => {
            // first pass the event through the interrupt registry. If any interrupt returns `Halt`, stop processing further.
            let mut ctx_template = Context::new(&mut store, 0, app_sender.clone(), system_sender.clone());
            if interrupt_registry.handle(event.as_ref(), &mut ctx_template) == InterruptResult::Halt {
              continue;
            }

            // event is only broadcast to systems if no interrupt halted it
            apply_to_systems(&mut systems, &mut store, &app_sender, &system_sender, |system, ctx| {
              system.on_event(event.as_ref(), ctx);
            });
          }
          AppMessage::SystemTick => {
            let tick_duration = config.system_timer_tick;
            apply_to_systems(&mut systems, &mut store, &app_sender, &system_sender, |system, ctx| {
              system.on_tick(tick_duration, ctx);
            });
            let mut ctx = Context::new(&mut store, 0, app_sender.clone(), system_sender.clone());
            machine.render_leds(&mut systems, config.system_timer_tick, &mut ctx).await;
          }
          AppMessage::RegisterInterrupt(system_id, type_id, priority, handler) => {
            interrupt_registry.register(type_id, system_id, priority, handler);
          }
          AppMessage::UnregisterInterrupt(system_id, type_id) => {
            interrupt_registry.unregister(system_id, type_id);
          }
          AppMessage::RegisterCommand(system_id, type_id, runner) => {
            command_registry.register(type_id, system_id, runner);
          }
          AppMessage::UnregisterCommand(_system_id, type_id) => {
            command_registry.unregister(type_id);
          }
          AppMessage::ExecuteCommand(system_id, cmd) => {
            // TODO: the context here is actually being set for the caller, not the system executing the command (which is wrong)
            let mut ctx = Context::new(&mut store, system_id, app_sender.clone(), system_sender.clone());
            command_registry.execute(&cmd, system_id, &mut ctx);
          }
          AppMessage::UnregisterAllBySystem(system_id) => {
            command_registry.unregister_by_system(system_id);
            interrupt_registry.unregister_by_system(system_id);
          }
          AppMessage::Shutdown => {
            log::info!("⏹️ Shutdown command received, shutting down...");
            break;
          }
          AppMessage::SwitchStates(switch_states) => {
            let switch_lookup = store.get_mut::<SwitchLookup>().unwrap();
            switch_lookup.update_switch_states(switch_states);
          }
        }
      }

      Some(command) = system_receiver.recv() => {
        match command {
          SystemMessage::SpawnSystem(system) => {
            SystemCommandsProcessor::spawn_system(system, &mut systems, &mut store, app_sender.clone(), system_sender.clone());
          }
          SystemMessage::ReplaceSystem(system_id, system) => {
            SystemCommandsProcessor::replace_system(system_id, system, &mut systems, &mut store, app_sender.clone(), system_sender.clone());
          }
          SystemMessage::DespawnSystem(system_id) => {
            SystemCommandsProcessor::despawn_system(system_id, &mut systems, &mut store, app_sender.clone(), system_sender.clone());
          }
          SystemMessage::ClearTimer(system_id, timer_name) => {
            SystemCommandsProcessor::clear_timer(system_id, timer_name, &mut systems);
          }
          SystemMessage::SetTimer(system_id, timer_name, duration, mode) => {
            SystemCommandsProcessor::set_timer(system_id, timer_name, duration, mode, &mut systems);
          }
        }
      }
    }
  }

  // Shutdown sequence
  apply_to_systems(
    &mut systems,
    &mut store,
    &app_sender,
    &system_sender,
    |system, ctx| {
      system.on_shutdown(ctx);
    },
  );

  // wait a sec to allow systems to process shutdown event and clear timers, etc.
  tokio::time::sleep(Duration::from_millis(1000)).await;
}

fn apply_to_systems<F>(
  systems: &mut Vec<SystemContainer>,
  store: &mut Store,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  system_sender: &mpsc::UnboundedSender<SystemMessage>,
  mut handler: F,
) where
  F: FnMut(&mut SystemContainer, &mut Context),
{
  for system in systems.iter_mut() {
    let mut ctx = Context::new(store, system.id, app_sender.clone(), system_sender.clone());
    if system.is_active(&ctx) {
      handler(system, &mut ctx);
    }
  }
}

// App Events
pub struct Shutdown;
