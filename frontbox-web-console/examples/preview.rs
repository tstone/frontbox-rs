use std::io::Write;

use frontbox::prelude::Event;
use frontbox::prelude::app_tracer::{AppTracer, TraceEvent};
use frontbox::prelude::event_box::EventBox;
use frontbox_web_console::WebTracer;

#[tokio::main]
async fn main() {
  env_logger::Builder::from_default_env()
    .format(|buf, record| writeln!(buf, "[{}] {}\r", record.level(), record.args()))
    .init();

  let tracer = WebTracer::new();
  let sender = tracer.sender();

  // generate some fake systems to preview what it looks like
  let _ = sender.send(TraceEvent::SystemGroupSpawned { key: "group1" });
  let _ = sender.send(TraceEvent::SystemGroupSpawned { key: "group2" });
  let _ = sender.send(TraceEvent::SystemGroupSpawned { key: "group3" });
  let _ = sender.send(TraceEvent::SystemSpawned {
    id: 0,
    name: "crate::frontbox-web-console::ExampleSystem1",
    parent_key: "group1",
  });
  let _ = sender.send(TraceEvent::SystemSpawned {
    id: 1,
    name: "crate::frontbox-web-console::ExampleSystem2",
    parent_key: "group2",
  });
  let _ = sender.send(TraceEvent::SystemSpawned {
    id: 2,
    name: "crate::frontbox-web-console::ExampleSystem3",
    parent_key: "group2",
  });
  let _ = sender.send(TraceEvent::SystemSpawned {
    id: 3,
    name: "crate::frontbox-web-console::ExampleSystem4",
    parent_key: "group3",
  });
  let _ = sender.send(TraceEvent::SystemActiveStateChange {
    id: 1,
    active: false,
  });
  let _ = sender.send(TraceEvent::SystemGroupActiveStateChange {
    key: "group3",
    active: false,
  });

  // ...and some fake events
  let event = EventBox::new(BodylessEvent);
  let _ = sender.send(TraceEvent::Event {
    type_name: event.type_name,
    interrupts: Vec::new(),
    event: event.try_json(),
  });
  let event = EventBox::new(TupleEvent(100));
  let _ = sender.send(TraceEvent::Event {
    type_name: event.type_name,
    interrupts: Vec::new(),
    event: event.try_json(),
  });
  let event = EventBox::new(FullEvent { id: 200 });
  let _ = sender.send(TraceEvent::Event {
    type_name: event.type_name,
    interrupts: Vec::new(),
    event: event.try_json(),
  });

  // keep web interface running
  loop {}
}

#[derive(serde::Serialize, Event)]
struct BodylessEvent;

#[derive(serde::Serialize, Event)]
struct TupleEvent(pub u32);

#[derive(serde::Serialize, Event)]
struct FullEvent {
  id: u64,
}
