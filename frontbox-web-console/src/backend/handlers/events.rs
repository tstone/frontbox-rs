use askama::Template;
use axum::extract::State;
use axum::response::{Html, IntoResponse};

use crate::backend::{AppState, SystemEvent};

pub async fn events_handler(State(state): State<AppState>) -> impl IntoResponse {
  let events = state
    .events
    .lock()
    .unwrap()
    .iter()
    .map(|e| e.clone())
    .collect();
  let template = EventsTemplate { events };
  Html(template.render().unwrap())
}

#[derive(Template)]
#[template(path = "events.html")]
struct EventsTemplate {
  events: Vec<SystemEvent>,
}
