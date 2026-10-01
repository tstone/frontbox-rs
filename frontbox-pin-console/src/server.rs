use axum::Router;
use axum::extract::{Path, State};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use rust_embed::RustEmbed;
use std::net::SocketAddr;
use tokio::sync::broadcast::error::RecvError;

use crate::console_hub::{ConsoleHub, Subscription};

/// The built SolidJS app. In debug builds rust-embed reads from disk, so `npm run build` is picked
/// up without recompiling. In release builds it's baked into the binary.
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
      log::error!(target: "frontbox_pin_console", "Unable to bind web console to {addr}: {err}");
      return;
    }
  };
  log::info!(target: "frontbox_pin_console", "Web console listening on http://{addr}");
  if let Err(err) = axum::serve(listener, app).await {
    log::error!(target: "frontbox_pin_console", "Web console stopped: {err}");
  }
}

async fn ws_handler(ws: WebSocketUpgrade, State(hub): State<ConsoleHub>) -> Response {
  ws.on_upgrade(move |socket| client_session(socket, hub))
}

async fn client_session(mut socket: WebSocket, hub: ConsoleHub) {
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
              target: "frontbox_pin_console",
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
        // nothing is expected from the client yet (future home of console -> machine
        // commands), but reading is how we notice it went away
        incoming = socket.recv() => match incoming {
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
      log::warn!(target: "frontbox_pin_console", "Unable to read plane image {}: {err}", path.display());
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
      "Web console has not been built. Run `npm run build` in frontbox-pin-console/web.",
    )
      .into_response(),
  }
}
