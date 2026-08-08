use axum::Json;
use axum::Router;
use axum::extract::Query;
use axum::extract::State;
use axum::routing::*;
use frontbox::prelude::app_tracer::TraceEvent;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tower_http::services::{ServeDir, ServeFile};

use crate::backend::handlers::events_handler;
use crate::backend::handlers::systems_handler;
use crate::backend::*;

#[derive(Clone)]
pub struct AppState {
  pub groups: Arc<Mutex<Vec<SystemGroup>>>,
  pub events: Arc<Mutex<VecDeque<SystemEvent>>>,
}

pub struct WebInterface;

impl WebInterface {
  pub fn new() -> Self {
    Self
  }

  pub async fn run(&self, mut rx: mpsc::UnboundedReceiver<TraceEvent>) {
    let groups: Arc<Mutex<Vec<SystemGroup>>> = Arc::new(Mutex::new(Vec::new()));
    let events: Arc<Mutex<VecDeque<SystemEvent>>> = Arc::new(Mutex::new(VecDeque::new()));

    // Collect incoming events from Frontbox into AppState
    let systems_writer = groups.clone();
    let events_writer = events.clone();
    tokio::spawn(async move {
      while let Some(event) = rx.recv().await {
        match event {
          TraceEvent::SystemGroupSpawned { key } => {
            let mut groups = systems_writer.lock().unwrap();
            groups.push(SystemGroup::new(key))
          }
          TraceEvent::SystemGroupDespawned { key } => {
            let mut groups = systems_writer.lock().unwrap();
            groups.retain(|g| g.key != key)
          }
          TraceEvent::SystemSpawned {
            id,
            name,
            parent_key,
          } => {
            let mut groups = systems_writer.lock().unwrap();
            if let Some(group) = groups.iter_mut().find(|g| g.key == parent_key) {
              group.systems.push(System::new(name, id));
            }
          }
          TraceEvent::SystemDespawned { id, parent_key } => {
            let mut groups = systems_writer.lock().unwrap();
            if let Some(group) = groups.iter_mut().find(|g| g.key == parent_key) {
              group.systems.retain(|s| s.id != id);
            }
          }
          TraceEvent::SystemActiveStateChange { id, active } => {
            let mut groups = systems_writer.lock().unwrap();
            for group in groups.iter_mut() {
              if let Some(system) = group.systems.iter_mut().find(|s| s.id == id) {
                system.active = active;
                break;
              }
            }
          }
          TraceEvent::SystemGroupActiveStateChange { key, active } => {
            let mut groups = systems_writer.lock().unwrap();
            if let Some(group) = groups.iter_mut().find(|g| g.key == key) {
              group.active = active;
            }
          }
          TraceEvent::Event {
            type_name,
            interrupts,
            event,
          } => {
            let mut events = events_writer.lock().unwrap();
            if events.len() > 128 {
              let _ = events.pop_front();
            }
            events.push_back(SystemEvent::new(type_name, interrupts, event));
          }
        }
      }
      log::info!("trace event channel closed, stopping log writer");
    });

    let public_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("public");
    let app = Router::new()
      .route("/events", get(events_handler))
      .route("/systems", get(systems_handler))
      .fallback_service(
        ServeDir::new(public_dir.clone()).fallback(ServeFile::new(public_dir.join("index.html"))),
      )
      .with_state(AppState { groups, events });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    log::info!("Listening on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
  }
}
