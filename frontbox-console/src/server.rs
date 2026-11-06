use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use rust_embed::RustEmbed;
use std::collections::HashSet;
use std::net::SocketAddr;
use tokio::sync::broadcast::error::RecvError;

use crate::console_hub::{ConsoleHub, Subscription};
use crate::protocol::ClientMessage;

/// The built SolidJS app
#[derive(RustEmbed)]
#[folder = "web/dist"]
#[allow_missing = true]
struct Assets;

pub(crate) async fn serve(hub: ConsoleHub, addr: SocketAddr) {
  let app = Router::new()
    .route("/ws", get(ws_handler))
    .route("/planes/{index}/image", get(plane_image))
    .fallback(static_handler)
    .with_state(hub);

  let listener = match tokio::net::TcpListener::bind(addr).await {
    Ok(listener) => listener,
    Err(err) => {
      log::error!(target: "frontbox_core::console", "Unable to bind web console to {addr}: {err}");
      return;
    }
  };
  log::info!(target: "frontbox_core::console", "Web console listening on http://{addr}");
  if let Err(err) = axum::serve(listener, app).await {
    log::error!(target: "frontbox_core::console", "Web console stopped: {err}");
  }
}

async fn ws_handler(ws: WebSocketUpgrade, State(hub): State<ConsoleHub>) -> Response {
  ws.on_upgrade(move |socket| client_session(socket, hub))
}

async fn client_session(mut socket: WebSocket, hub: ConsoleHub) {
  // outlives resyncs; dropped when the session ends for any reason
  let mut held = HeldSwitches::new(hub.clone());
  // a clip takes a while, and the session carries on in the meantime rather than waiting on it
  let (clip_sender, mut clips) = tokio::sync::mpsc::channel(1);
  'resync: loop {
    let Some(Subscription {
      init,
      events: mut rx,
      mut leds,
    }) = hub.subscribe()
    else {
      return;
    };
    if socket.send(Message::Text(init)).await.is_err() {
      return;
    }

    loop {
      tokio::select! {
        msg = rx.recv() => match msg {
          Ok(text) => {
            if socket.send(Message::Text(text)).await.is_err() {
              return;
            }
          }
          // client fell too far behind; start it over from a fresh snapshot
          Err(RecvError::Lagged(missed)) => {
            log::warn!(
              target: "frontbox_core::console",
              "A console client fell {missed} messages behind; sending it everything again"
            );
            continue 'resync;
          }
          Err(RecvError::Closed) => return,
        },

        led_update = leds.recv() => {
          let text = match led_update {
            Ok(text) => Some(text),
            // LED updates only carry what changed, so after skipping some, send every LED's current color
            Err(RecvError::Lagged(_)) => hub.all_leds(),
            Err(RecvError::Closed) => return,
          };
          if let Some(text) = text
            && socket.send(Message::Text(text)).await.is_err()
          {
            return;
          }
        }

        Some(clip) = clips.recv() => {
          if socket.send(clip).await.is_err() {
            return;
          }
        }

        incoming = socket.recv() => match incoming {
          Some(Ok(Message::Text(text))) => match serde_json::from_str::<ClientMessage>(&text) {
            // answered to this client alone; the query runs off the runtime, so it only holds up this session
            Ok(ClientMessage::QueryMetrics) => {
              if let Some(reply) = hub.metric_totals().await
                && socket.send(Message::Text(reply)).await.is_err()
              {
                return;
              }
            }
            Ok(ClientMessage::RenderScene { source, size, leds, time, cycle }) => {
              if let Some(reply) = hub.render_scene(source, size, leds, time, cycle).await
                && socket.send(Message::Text(reply)).await.is_err()
              {
                return;
              }
            }
            Ok(ClientMessage::RenderClip { id, source, size, leds, cycle }) => {
              let hub = hub.clone();
              let clip_sender = clip_sender.clone();
              tokio::spawn(async move {
                if let Some(clip) = hub.render_clip(id, source, size, leds, cycle).await {
                  let _ = clip_sender.send(clip).await;
                }
              });
            }
            Ok(message) => held.handle(message),
            Err(err) => log::warn!(target: "frontbox_core::console", "Ignoring a message from a console client: {err}"),
          },
          Some(Ok(_)) => {}
          _ => return,
        },
      }
    }
  }
}

/// A plane's image, read from disk on each request so it can be edited while the machine runs
async fn plane_image(Path(index): Path<usize>, State(hub): State<ConsoleHub>) -> Response {
  let Some(path) = hub.plane_image(index) else {
    return StatusCode::NOT_FOUND.into_response();
  };
  match tokio::fs::read(&path).await {
    Ok(bytes) => {
      let mime = mime_guess::from_path(&path).first_or_octet_stream();
      ([(header::CONTENT_TYPE, mime.to_string())], bytes).into_response()
    }
    Err(err) => {
      log::warn!(target: "frontbox_core::console", "Unable to read plane image {}: {err}", path.display());
      StatusCode::NOT_FOUND.into_response()
    }
  }
}

async fn static_handler(uri: Uri) -> Response {
  let path = uri.path().trim_start_matches('/');
  let path = if path.is_empty() { "index.html" } else { path };

  match Assets::get(path).or_else(|| Assets::get("index.html")) {
    Some(file) => (
      [(header::CONTENT_TYPE, file.metadata.mimetype().to_string())],
      file.data,
    )
      .into_response(),
    None => (
      StatusCode::NOT_FOUND,
      "Web console has not been built. Run `npm run build` in frontbox-console/web.",
    )
      .into_response(),
  }
}

/// Switches a client has closed. When its session ends they're opened again, so a closed tab can't leave a switch
/// held closed in the game.
struct HeldSwitches {
  hub: ConsoleHub,
  closed: HashSet<usize>,
}

impl HeldSwitches {
  fn new(hub: ConsoleHub) -> Self {
    Self {
      hub,
      closed: HashSet::new(),
    }
  }

  fn handle(&mut self, message: ClientMessage) {
    match message {
      ClientMessage::SetSwitch { switch_id, closed } => {
        if closed {
          self.closed.insert(switch_id);
        } else {
          self.closed.remove(&switch_id);
        }
        self.hub.set_switch(switch_id, closed);
      }
      ClientMessage::SetConfig { key, value } => self.hub.set_config(key, value),
      // replied to by the session, which has the socket
      ClientMessage::QueryMetrics | ClientMessage::RenderScene { .. } | ClientMessage::RenderClip { .. } => {}
    }
  }
}

impl Drop for HeldSwitches {
  fn drop(&mut self) {
    for switch_id in self.closed.drain() {
      self.hub.set_switch(switch_id, false);
    }
  }
}
