use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use frontbox_gfx::scene::{Content, ContentError, SceneFileError};
use rust_embed::RustEmbed;
use std::collections::HashSet;
use std::net::SocketAddr;
use tokio::sync::broadcast::error::RecvError;

use crate::console_hub::{ConsoleHub, Subscription};
use crate::protocol::{ClientMessage, ImageInfo, SceneSource};
use crate::scene_render::scene_file;

/// The largest image a client can add
const MAX_IMAGE_BYTES: usize = 64 * 1024 * 1024;

const NO_CONTENT_FOLDER: &str = "this machine has no content folder";

/// The built SolidJS app
#[derive(RustEmbed)]
#[folder = "web/dist"]
#[allow_missing = true]
struct Assets;

pub(crate) async fn serve(hub: ConsoleHub, addr: SocketAddr) {
  let app = Router::new()
    .route("/ws", get(ws_handler))
    .route("/planes/{index}/image", get(plane_image))
    .route("/content/images", get(content_images))
    .route(
      "/content/images/{*name}",
      get(content_image).put(add_content_image).layer(DefaultBodyLimit::max(MAX_IMAGE_BYTES)),
    )
    .route("/content/image-info/{*name}", get(content_image_info))
    .route("/content/sounds", get(content_sounds))
    .route("/content/sounds/{*name}", get(content_sound))
    .route("/content/scenes", get(content_scenes))
    .route(
      "/content/scenes/{*name}",
      get(content_scene).post(add_content_scene).put(save_content_scene).delete(delete_content_scene),
    )
    .fallback(static_handler)
    .with_state(hub);

  let listener = match tokio::net::TcpListener::bind(addr).await {
    Ok(listener) => listener,
    Err(err) => {
      log::error!(target: "frontbox::console", "Unable to bind web console to {addr}: {err}");
      return;
    }
  };
  log::info!(target: "frontbox::console", "Web console listening on http://{addr}");
  if let Err(err) = axum::serve(listener, app).await {
    log::error!(target: "frontbox::console", "Web console stopped: {err}");
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
      mut displays,
    }) = hub.subscribe()
    else {
      return;
    };
    if socket.send(Message::Text(init)).await.is_err() {
      return;
    }
    for frame in hub.display_frames() {
      if socket.send(Message::Binary(frame)).await.is_err() {
        return;
      }
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
              target: "frontbox::console",
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

        frame = displays.recv() => {
          let frames = match frame {
            Ok(frame) => vec![frame],
            // each is a whole frame, so after skipping some, the newest of each display is all that's needed
            Err(RecvError::Lagged(_)) => hub.display_frames(),
            Err(RecvError::Closed) => return,
          };
          for frame in frames {
            if socket.send(Message::Binary(frame)).await.is_err() {
              return;
            }
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
            Ok(ClientMessage::RenderScene { source, size, time, cycle }) => {
              if let Some(reply) = hub.render_scene(source, size, time, cycle).await
                && socket.send(Message::Text(reply)).await.is_err()
              {
                return;
              }
            }
            Ok(ClientMessage::RenderClip { id, source, size, cycle }) => {
              let hub = hub.clone();
              let clip_sender = clip_sender.clone();
              tokio::spawn(async move {
                if let Some(clip) = hub.render_clip(id, source, size, cycle).await {
                  let _ = clip_sender.send(clip).await;
                }
              });
            }
            Ok(message) => held.handle(message),
            Err(err) => log::warn!(target: "frontbox::console", "Ignoring a message from a console client: {err}"),
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
      log::warn!(target: "frontbox::console", "Unable to read plane image {}: {err}", path.display());
      StatusCode::NOT_FOUND.into_response()
    }
  }
}

/// The name of every image in the content folder. An app without one has none.
async fn content_images(State(hub): State<ConsoleHub>) -> Response {
  let Some(content) = hub.content() else {
    return Json(Vec::<String>::new()).into_response();
  };
  match tokio::task::spawn_blocking(move || content.image_names()).await {
    Ok(names) => Json(names).into_response(),
    Err(err) => {
      log::error!(target: "frontbox::console", "Scanning the content folder stopped: {err}");
      StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
  }
}

/// An image from the content folder, read from disk on each request so it can be edited while the machine runs
async fn content_image(Path(name): Path<String>, State(hub): State<ConsoleHub>) -> Response {
  let Some(Ok(path)) = hub.content().map(|content| content.image_path(&name)) else {
    return StatusCode::NOT_FOUND.into_response();
  };
  match tokio::fs::read(&path).await {
    Ok(bytes) => {
      let mime = mime_guess::from_path(&path).first_or_octet_stream();
      ([(header::CONTENT_TYPE, mime.to_string())], bytes).into_response()
    }
    Err(_) => StatusCode::NOT_FOUND.into_response(),
  }
}

/// The size and frames of an image in the content folder. This decodes it if nothing has yet. The reply to a failure is
/// text for showing to whoever chose it.
async fn content_image_info(Path(name): Path<String>, State(hub): State<ConsoleHub>) -> Response {
  let Some(content) = hub.content() else {
    return StatusCode::NOT_FOUND.into_response();
  };
  match tokio::task::spawn_blocking(move || content.picture(&name)).await {
    Ok(Ok(picture)) => Json(ImageInfo {
      width: picture.width(),
      height: picture.height(),
      frames: picture.frames(),
      duration: picture.duration(),
    })
    .into_response(),
    Ok(Err(ContentError::Decode { name, error })) => {
      (StatusCode::UNPROCESSABLE_ENTITY, ContentError::Decode { name, error }.to_string()).into_response()
    }
    Ok(Err(err)) => (StatusCode::NOT_FOUND, err.to_string()).into_response(),
    Err(err) => {
      log::error!(target: "frontbox::console", "Reading an image stopped: {err}");
      StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
  }
}

/// Adds an image to the content folder. The reply to a failure is text for showing to whoever is adding it.
async fn add_content_image(Path(name): Path<String>, State(hub): State<ConsoleHub>, bytes: Bytes) -> Response {
  let Some(content) = hub.content() else {
    return (StatusCode::CONFLICT, NO_CONTENT_FOLDER).into_response();
  };
  let added = tokio::task::spawn_blocking(move || content.add_image(&name, &bytes)).await;
  match added {
    Ok(Ok(())) => StatusCode::CREATED.into_response(),
    Ok(Err(err)) => {
      let status = match err {
        ContentError::Taken(_) => StatusCode::CONFLICT,
        ContentError::Write { .. } | ContentError::Read { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        ContentError::BadName(_) | ContentError::Decode { .. } => StatusCode::UNPROCESSABLE_ENTITY,
      };
      (status, err.to_string()).into_response()
    }
    Err(err) => {
      log::error!(target: "frontbox::console", "Adding an image stopped: {err}");
      StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
  }
}

/// The name of every sound in the content folder. An app without one has none.
async fn content_sounds(State(hub): State<ConsoleHub>) -> Response {
  let Some(content) = hub.content() else {
    return Json(Vec::<String>::new()).into_response();
  };
  match tokio::task::spawn_blocking(move || content.sound_names()).await {
    Ok(names) => Json(names).into_response(),
    Err(err) => {
      log::error!(target: "frontbox::console", "Scanning the content folder stopped: {err}");
      StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
  }
}

/// A sound from the content folder, read from disk on each request so it can be edited while the machine runs
async fn content_sound(Path(name): Path<String>, State(hub): State<ConsoleHub>) -> Response {
  let Some(Ok(path)) = hub.content().map(|content| content.sound_path(&name)) else {
    return StatusCode::NOT_FOUND.into_response();
  };
  match tokio::fs::read(&path).await {
    Ok(bytes) => {
      let mime = mime_guess::from_path(&path).first_or_octet_stream();
      ([(header::CONTENT_TYPE, mime.to_string())], bytes).into_response()
    }
    Err(_) => StatusCode::NOT_FOUND.into_response(),
  }
}

/// The name of every scene in the content folder. An app without one has none.
async fn content_scenes(State(hub): State<ConsoleHub>) -> Response {
  let Some(content) = hub.content() else {
    return Json(Vec::<String>::new()).into_response();
  };
  match tokio::task::spawn_blocking(move || content.scene_names()).await {
    Ok(names) => Json(names).into_response(),
    Err(err) => {
      log::error!(target: "frontbox::console", "Scanning the content folder stopped: {err}");
      StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
  }
}

/// A scene's file as RON, read from disk on each request so it can be edited while the machine runs. The reply to a
/// failure is text for showing to whoever is opening it.
async fn content_scene(Path(name): Path<String>, State(hub): State<ConsoleHub>) -> Response {
  let Some(content) = hub.content() else {
    return (StatusCode::CONFLICT, NO_CONTENT_FOLDER).into_response();
  };
  match tokio::task::spawn_blocking(move || content.read_scene(&name)).await {
    Ok(Ok(ron)) => ron.into_response(),
    Ok(Err(err)) => scene_file_failure(err),
    Err(err) => {
      log::error!(target: "frontbox::console", "Reading a scene stopped: {err}");
      StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
  }
}

/// Adds a scene to the content folder. The reply is the file as it's been written, or text for showing to whoever is
/// adding it.
async fn add_content_scene(hub: State<ConsoleHub>, name: Path<String>, source: Json<SceneSource>) -> Response {
  write_scene(hub, name, source, StatusCode::CREATED, Content::add_scene).await
}

/// Saves a scene over the one in the content folder. Replies as `add_content_scene` does.
async fn save_content_scene(hub: State<ConsoleHub>, name: Path<String>, source: Json<SceneSource>) -> Response {
  write_scene(hub, name, source, StatusCode::OK, Content::save_scene).await
}

/// Removes a scene from the content folder. The reply to a failure is text for showing to whoever is deleting it.
async fn delete_content_scene(Path(name): Path<String>, State(hub): State<ConsoleHub>) -> Response {
  let Some(content) = hub.content() else {
    return (StatusCode::CONFLICT, NO_CONTENT_FOLDER).into_response();
  };
  match tokio::task::spawn_blocking(move || content.delete_scene(&name)).await {
    Ok(Ok(())) => StatusCode::NO_CONTENT.into_response(),
    Ok(Err(err)) => scene_file_failure(err),
    Err(err) => {
      log::error!(target: "frontbox::console", "Deleting a scene stopped: {err}");
      StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
  }
}

async fn write_scene(
  State(hub): State<ConsoleHub>,
  Path(name): Path<String>,
  Json(source): Json<SceneSource>,
  written: StatusCode,
  write: fn(&Content, &str, &str) -> Result<(), SceneFileError>,
) -> Response {
  let Some(content) = hub.content() else {
    return (StatusCode::CONFLICT, NO_CONTENT_FOLDER).into_response();
  };
  let ron = match scene_file(source) {
    Ok(ron) => ron,
    Err(err) => return (StatusCode::UNPROCESSABLE_ENTITY, err).into_response(),
  };
  match tokio::task::spawn_blocking(move || write(&content, &name, &ron).map(|()| ron)).await {
    Ok(Ok(ron)) => (written, ron).into_response(),
    Ok(Err(err)) => scene_file_failure(err),
    Err(err) => {
      log::error!(target: "frontbox::console", "Writing a scene stopped: {err}");
      StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
  }
}

fn scene_file_failure(err: SceneFileError) -> Response {
  let status = match err {
    SceneFileError::Read { ref error, .. } | SceneFileError::Delete { ref error, .. }
      if error.kind() == std::io::ErrorKind::NotFound =>
    {
      StatusCode::NOT_FOUND
    }
    SceneFileError::Taken(_) => StatusCode::CONFLICT,
    SceneFileError::Read { .. } | SceneFileError::Write { .. } | SceneFileError::Delete { .. } => {
      StatusCode::INTERNAL_SERVER_ERROR
    }
    SceneFileError::BadName(_) => StatusCode::UNPROCESSABLE_ENTITY,
  };
  (status, err.to_string()).into_response()
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
      ClientMessage::PlaySound { sound, volume, skip, length } => self.hub.play_sound(sound, volume, skip, length),
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
