use askama::Template;
use axum::extract::State;
use axum::response::{Html, IntoResponse};

use crate::backend::{AppState, SystemGroup};

pub async fn systems_handler(State(state): State<AppState>) -> impl IntoResponse {
  let groups: Vec<SystemGroup> = state.groups.lock().unwrap().clone();
  let template = SystemsTemplate { groups };
  Html(template.render().unwrap())
}

#[derive(Template)]
#[template(path = "systems.html")]
struct SystemsTemplate {
  groups: Vec<SystemGroup>,
}
