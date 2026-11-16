use std::collections::HashMap;

use frontbox_core::prelude::*;
use frontbox_gfx::scene::{Prop, Transform};
use frontbox_scene::OpenError;
use frontbox_scene::prelude::*;
#[cfg(feature = "sound")]
use frontbox_sound::*;

use crate::menu::{MenuRow, MenuSection};
use crate::*;

const SELECT_SND: &str = "dmd_menu_select";
const INC_SOUND: &str = "dmd_menu_inc";
const DEC_SOUND: &str = "dmd_menu_dec";
const BACK_SND: &str = "dmd_menu_back";
const NOT_ALLOWED_SND: &str = "dmd_menu_not_allowed";
const MENU_BEGIN_SND: &str = "dmd_menu_begin";
const MENU_END_SND: &str = "dmd_menu_end";

/// Rows shown at once
const ROWS_SHOWN: usize = 4;
/// How far apart rows are, in pixels. A row's fragment draws the first row, and each one below it is moved down by this.
const ROW_HEIGHT: f32 = 7.0;
/// How long the selection takes to slide to another row
const SLIDE_DURATION: Duration = Duration::from_millis(120);

/// The operator menu, shown on the DMD while the coin door is open. It's drawn from fragments: scenes for its parts,
/// `frame`, `selection`, `heading`, `row`, `edit`, and `about`, in `frontbox/128x32/menu/` unless
/// [`fragments`](Self::fragments) names another folder. A game changes how the menu looks by putting its own files
/// under those names in its content folder.
///
/// Each part is played as a copy of its fragment, moved down to its row. The copies are drawn frame first, then the
/// selection, then the rows from the top. The code sets their variables:
///
/// - `selection`: `offset`, how far it's moved from the first row to the selected one. It slides there when the
///   selection moves.
/// - `heading`: `text`
/// - `row`: `label`, `value`, `opens` (it opens a section, screen, or action, so it shows the arrow), `has_value`
///   (it's a setting), and `changed` (a setting that isn't at its default). Each variable whose default points at a
///   value named `selected_…` or `normal_…` is pointed at the one for whether the row is selected, so a fragment can
///   add to what selection changes without code.
/// - `edit`: `label`, `description`, `value`, and `changed`
/// - `about`: `version`
pub struct DmdMenuSystem {
  switch_names: MenuSwitches,
  /// The folder the fragments are in
  folder: String,
  /// Of every layer it draws
  z: i8,
  /// Opened when the system spawns. `None` if any of them couldn't be, and then nothing is drawn.
  fragments: Option<Fragments>,
  /// What's on the display, in the order it's drawn
  shown: Vec<ShownPart>,
  row_lookup: HashMap<u64, MenuRowReference>,
  active_section: &'static MenuSection,
  display_rows: Vec<DisplayMenuRow>,
  selected_row: usize,
  selected_config: Option<&'static MenuRow>,
  selected_special: Option<&'static str>,
  // indicates that the rows vector needs to be re-generated because something changed
  requires_row_refresh: bool,
  // indicates that something about the display changed and needs to be re-rendered
  requires_render: bool,
}

impl DmdMenuSystem {
  pub fn new(switch_names: MenuSwitches, root: &'static MenuSection) -> Self {
    let root: &'static MenuSection = {
      let mut root = root.clone();
      root
        .rows
        .push(MenuRow::Special(MenuRow::next_row_id(), "About"));
      Box::leak(Box::new(root))
    };
    let root_row: &'static MenuRow = Box::leak(Box::new(MenuRow::Section(root.clone())));

    // Build up a quick lookup table by ID to row/parent
    let mut row_lookup = HashMap::<u64, MenuRowReference>::new();
    row_lookup.insert(
      root.id,
      MenuRowReference {
        row: root_row,
        parent: None,
      },
    );
    Self::build_row_lookup(&mut row_lookup, root);

    Self {
      switch_names,
      folder: "frontbox/128x32/menu".into(),
      z: i8::MAX,
      fragments: None,
      shown: Vec::new(),
      row_lookup,
      active_section: root,
      display_rows: Vec::new(),
      selected_row: Self::first_non_header_row(root),
      selected_config: None,
      selected_special: None,
      requires_row_refresh: true,
      requires_render: true,
    }
  }

  /// Reads the fragments from `folder` in the content folder or among the bundled scenes, by the names
  /// [`DmdMenuSystem`] lists
  pub fn fragments(mut self, folder: impl Into<String>) -> Self {
    self.folder = folder.into();
    self
  }

  /// Draws the menu at `z` among everything on the display. It's on top of everything unless this says otherwise.
  pub fn z(mut self, z: i8) -> Self {
    self.z = z;
    self
  }

  /// Build a quick id => section, section parent lookup table for fast navigation
  fn build_row_lookup(lookup: &mut HashMap<u64, MenuRowReference>, parent: &'static MenuSection) {
    for row in &parent.rows {
      lookup.insert(
        row.id(),
        MenuRowReference {
          row,
          parent: Some(parent),
        },
      );

      if let MenuRow::Section(section) = row {
        Self::build_row_lookup(lookup, section);
      }
    }
  }

  fn first_non_header_row(section: &'static MenuSection) -> usize {
    section
      .rows
      .iter()
      .position(|r| !matches!(r, MenuRow::Heading(_, _)))
      .unwrap_or(0)
  }

  fn activate_section(&mut self, section: &'static MenuSection, ctx: &SystemContext) {
    self.active_section = section;
    self.selected_row = Self::first_non_header_row(self.active_section);
    self.refresh_display_rows(ctx);
  }

  fn navigate_fwd(&mut self, ctx: &SystemContext) {
    match self.display_rows[self.selected_row] {
      DisplayMenuRow::Section { id, .. } => {
        let section = self.row_lookup.get(&id).unwrap().row.section().unwrap();
        self.activate_section(section, ctx);
        ctx.play_sfx(SELECT_SND);
      }
      DisplayMenuRow::Config { id, .. } => {
        if let Some(reference) = self.row_lookup.get(&id) {
          self.selected_config = Some(reference.row);
          self.requires_render = true;
          ctx.play_sfx(SELECT_SND);
        }
      }
      DisplayMenuRow::Special(name) => {
        self.selected_special = Some(name);
        self.requires_render = true;
        ctx.play_sfx(SELECT_SND);
      }
      DisplayMenuRow::Action { id, .. } => {
        if let Some(reference) = self.row_lookup.get(&id) {
          if let MenuRow::Action(_, _, action) = reference.row {
            ctx.play_sfx(SELECT_SND);
            action(ctx);
          }
        }
      }
      _ => {}
    }
  }

  fn navigate_back(&mut self, ctx: &SystemContext) {
    if self.selected_config.is_some() {
      self.selected_config = None;
      self.requires_render = true;
      ctx.play_sfx(BACK_SND);
    } else if self.selected_special.is_some() {
      self.selected_special = None;
      self.requires_render = true;
      ctx.play_sfx(BACK_SND);
    } else {
      let parent = self
        .row_lookup
        .get(&self.active_section.id)
        .and_then(|e| e.parent);
      if let Some(parent) = parent {
        ctx.play_sfx(BACK_SND);
        self.activate_section(parent, ctx);
      } else {
        ctx.play_sfx(NOT_ALLOWED_SND);
      }
    }
  }

  fn navigate_inc(&mut self, ctx: &SystemContext) {
    if let Some(row) = &mut self.selected_config {
      if let Some(value) = self
        .display_rows
        .iter_mut()
        .find(|r| r.id() == row.id())
        .unwrap()
        .value_mut()
      {
        *value = row.increment(ctx);
        log::debug!(target: "frontbox::display", "increment: {}", value);
      }
      self.requires_render = true;
    } else {
      let mut next = self.selected_row;

      loop {
        next += 1;

        match self.display_rows.get(next) {
          // skip headings during increment
          Some(DisplayMenuRow::Heading { .. }) if next >= self.display_rows.len() => {
            next = self.selected_row;
            break;
          }
          Some(DisplayMenuRow::Heading { .. }) => continue,
          _ if next >= self.display_rows.len() => {
            next = self.selected_row;
            break;
          }
          _ => break,
        }
      }

      if self.selected_row != next {
        self.selected_row = next;
        self.requires_render = true;
      }
    }
    ctx.play_sfx(INC_SOUND);
  }

  fn navigate_dec(&mut self, ctx: &SystemContext) {
    if let Some(row) = &self.selected_config {
      if let Some(value) = self
        .display_rows
        .iter_mut()
        .find(|r| r.id() == row.id())
        .unwrap()
        .value_mut()
      {
        *value = row.decrement(ctx);
      }
      self.requires_render = true;
    } else if self.selected_row > 0 {
      let mut prev = self.selected_row;

      loop {
        prev -= 1;

        match self.display_rows.get(prev) {
          // skip headings during increment
          Some(DisplayMenuRow::Heading { .. }) if prev == 0 => {
            prev = self.selected_row;
            break;
          }
          Some(DisplayMenuRow::Heading { .. }) => continue,
          _ => break,
        }
      }

      if self.selected_row != prev {
        self.selected_row = prev;
        self.requires_render = true;
      }
    }

    ctx.play_sfx(DEC_SOUND);
  }

  // Display rows contained memoized values which may change. Upon trigger, re-generate these rows with the latest values
  fn refresh_display_rows(&mut self, ctx: &SystemContext) {
    self.display_rows.clear();

    for row in &self.active_section.rows {
      match row {
        MenuRow::Section(section) => {
          self.display_rows.push(DisplayMenuRow::Section {
            id: section.id,
            name: section.name.clone(),
          });
        }
        MenuRow::Config(id, config) => {
          let value = ctx.config.inspect(config);
          self.display_rows.push(DisplayMenuRow::Config {
            id: *id,
            name: config.label,
            desc: config.desc,
            changed: !config.is_default(value),
            value: config.format(value),
          });
        }
        MenuRow::Heading(id, text) => {
          self.display_rows.push(DisplayMenuRow::Heading {
            id: *id,
            text: text.to_string(),
          });
        }
        MenuRow::Special(_, text) => self.display_rows.push(DisplayMenuRow::Special(text)),
        MenuRow::Action(id, text, _) => self
          .display_rows
          .push(DisplayMenuRow::Action { id: *id, text }),
      }
    }

    self.requires_render = true;
  }

  /// Plays the parts for what the menu shows now, keeping the playbacks of parts that are still in the same place
  fn show(&mut self, ctx: &SystemContext) {
    let Some(fragments) = &self.fragments else {
      return;
    };
    let parts = self.parts();
    let count = parts.len();
    for (index, part) in parts.into_iter().enumerate() {
      let same_place = self
        .shown
        .get(index)
        .is_some_and(|shown| shown.fragment == part.fragment && shown.row == part.row);
      if !same_place {
        let scene = placed(fragments.scene(part.fragment), part.row, self.z);
        let player = ScenePlayer::started(PlayableScene::new(scene), ctx);
        let shown = ShownPart {
          fragment: part.fragment,
          row: part.row,
          player,
        };
        match self.shown.get_mut(index) {
          Some(old) => std::mem::replace(old, shown).player.stop(ctx),
          None => self.shown.push(shown),
        }
      }
      let player = &mut self.shown[index].player;
      for (name, value) in part.variables {
        player.set_variable(name, value, ctx);
      }
      for (name, value) in part.eased {
        if same_place {
          player.ease_variable(name, value, SLIDE_DURATION, Curve::EaseOut, ctx);
        } else {
          player.set_variable(name, value, ctx);
        }
      }
    }
    for mut gone in self.shown.drain(count..) {
      gone.player.stop(ctx);
    }
  }

  /// What the menu shows now, in the order it's drawn
  fn parts(&self) -> Vec<Part> {
    if let Some(row) = self.selected_config
      && let Some(DisplayMenuRow::Config {
        name,
        desc,
        value,
        changed,
        ..
      }) = self.display_rows.iter().find(|r| r.id() == row.id())
    {
      return vec![
        Part::new(Fragment::Edit, 0)
          .variable("label", *name)
          .variable("description", *desc)
          .variable("value", value.as_str())
          .variable("changed", *changed),
      ];
    }
    if self.selected_special.is_some() {
      return vec![Part::new(Fragment::About, 0).variable("version", env!("CARGO_PKG_VERSION"))];
    }

    let first = self
      .selected_row
      .saturating_sub(2)
      .min(self.display_rows.len().saturating_sub(ROWS_SHOWN));
    let rows = self
      .display_rows
      .iter()
      .enumerate()
      .skip(first)
      .take(ROWS_SHOWN);
    let selection = Vec2::new(0.0, (self.selected_row - first) as f32 * ROW_HEIGHT);
    let mut parts = vec![
      Part::new(Fragment::Frame, 0),
      Part::new(Fragment::Selection, 0).eased("offset", selection),
    ];
    for (row, (index, display_row)) in rows.enumerate() {
      let part = match display_row {
        DisplayMenuRow::Heading { text, .. } => {
          Part::new(Fragment::Heading, row).variable("text", text.as_str())
        }
        DisplayMenuRow::Config {
          name,
          value,
          changed,
          ..
        } => Part::new(Fragment::Row, row).row(name, value, false, *changed),
        DisplayMenuRow::Section { name, .. } => {
          Part::new(Fragment::Row, row).row(name, "", true, false)
        }
        DisplayMenuRow::Special(name) => Part::new(Fragment::Row, row).row(name, "", true, false),
        DisplayMenuRow::Action { text, .. } => {
          Part::new(Fragment::Row, row).row(text, "", true, false)
        }
      };
      let selected = index == self.selected_row;
      parts.push(match &self.fragments {
        Some(fragments) if part.fragment == Fragment::Row => {
          part.selected(&fragments.row, selected)
        }
        _ => part,
      });
    }
    parts
  }
}

impl System for DmdMenuSystem {
  fn is_active(&self, ctx: &SystemContext) -> bool {
    ctx
      .switches
      .is_open(self.switch_names.coin_door)
      .unwrap_or(false)
  }

  fn on_spawn(&mut self, ctx: &SystemContext) {
    let content = content_with_bundled(ctx.content_path.clone());
    match Fragments::open(&content, &self.folder) {
      Ok(fragments) => self.fragments = Some(fragments),
      Err(error) => log::error!(target: "frontbox::display", "The menu can't be drawn: {error}"),
    }

    if let Some(mut snd) = ctx.systems.get::<SoundSystem>() {
      snd.preload_embedded(MENU_BEGIN_SND, menu::sounds::DOOR_OPEN);
      snd.preload_embedded(BACK_SND, menu::sounds::MENU_BACK);
      snd.preload_embedded(SELECT_SND, menu::sounds::MENU_FWD);
      snd.preload_embedded(INC_SOUND, menu::sounds::MENU_UP);
      snd.preload_embedded(DEC_SOUND, menu::sounds::MENU_DOWN);
    }

    // catch the case where the coin door may already be open when the system starts
    if ctx
      .switches
      .is_open(self.switch_names.coin_door)
      .unwrap_or(false)
    {
      self.on_reactivate(ctx);
    }
  }

  fn on_reactivate(&mut self, ctx: &SystemContext) {
    self.requires_row_refresh = true;
    ctx.play_sfx(MENU_BEGIN_SND);
  }

  fn on_deactivate(&mut self, ctx: &SystemContext) {
    ctx.play_sfx(MENU_END_SND);
  }

  fn on_tick(&mut self, _delta: Duration, ctx: &SystemContext) {
    if self.requires_row_refresh {
      self.refresh_display_rows(ctx);
      self.requires_row_refresh = false;
    }

    if self.requires_render {
      self.show(ctx);
      self.requires_render = false;
    }
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(event) = event.downcast_ref::<SwitchClosed>() {
      if event.switch.name == self.switch_names.back_btn {
        self.navigate_back(ctx);
      } else if event.switch.name == self.switch_names.select_btn {
        self.navigate_fwd(ctx);
      } else if event.switch.name == self.switch_names.inc_btn {
        self.navigate_inc(ctx);
      } else if event.switch.name == self.switch_names.dec_btn {
        self.navigate_dec(ctx);
      }
    }
  }
}

/// The counterpart to MenuRow which only handles display data
enum DisplayMenuRow {
  Section {
    id: u64,
    name: String,
  },
  Config {
    id: u64,
    name: &'static str,
    desc: &'static str,
    value: String,
    changed: bool,
  },
  Heading {
    id: u64,
    text: String,
  },
  Special(&'static str),
  Action {
    id: u64,
    text: &'static str,
  },
}

impl DisplayMenuRow {
  pub fn id(&self) -> u64 {
    match self {
      DisplayMenuRow::Config { id, .. } => *id,
      DisplayMenuRow::Section { id, .. } => *id,
      DisplayMenuRow::Heading { id, .. } => *id,
      DisplayMenuRow::Special(_) => 0,
      DisplayMenuRow::Action { id, .. } => *id,
    }
  }

  pub fn value_mut(&mut self) -> Option<&mut String> {
    match self {
      DisplayMenuRow::Config { value, .. } => Some(value),
      _ => None,
    }
  }
}

pub struct MenuSwitches {
  pub coin_door: &'static str,
  pub back_btn: &'static str,
  pub select_btn: &'static str,
  pub inc_btn: &'static str,
  pub dec_btn: &'static str,
}

struct MenuRowReference {
  row: &'static MenuRow,
  parent: Option<&'static MenuSection>,
}

/// A copy of `scene` moved down to `row`, with every display layer at `z`. A layer without a region covers the whole
/// display, so it stays where it is.
fn placed(scene: &Scene, row: usize, z: i8) -> Scene {
  let mut scene = scene.clone();
  let offset = Vec2::new(0.0, row as f32 * ROW_HEIGHT);
  for layer in scene
    .layers
    .iter_mut()
    .filter_map(SceneLayer::as_display_mut)
  {
    layer.z = z;
    if let Some(region) = &mut layer.layer.region
      && row > 0
    {
      region
        .transform
        .push(Transform::Translate(Prop::Fixed(offset)));
    }
  }
  scene
}

/// The menu's fragments, opened from the content folder or the bundled scenes
struct Fragments {
  frame: Scene,
  selection: Scene,
  heading: Scene,
  row: Scene,
  edit: Scene,
  about: Scene,
}

impl Fragments {
  fn open(content: &Content, folder: &str) -> Result<Self, OpenError> {
    let open = |name: &str, sends: Sends| {
      let path = format!("{folder}/{name}.scene");
      let scene = Scene::open(content, &path)?;
      sends.log_mismatches(&path, &scene);
      Ok(scene)
    };
    Ok(Self {
      frame: open("frame", Sends { variables: &[] })?,
      selection: open("selection", SELECTION_SENDS)?,
      heading: open("heading", HEADING_SENDS)?,
      row: open("row", ROW_SENDS)?,
      edit: open("edit", EDIT_SENDS)?,
      about: open("about", ABOUT_SENDS)?,
    })
  }

  fn scene(&self, fragment: Fragment) -> &Scene {
    match fragment {
      Fragment::Frame => &self.frame,
      Fragment::Selection => &self.selection,
      Fragment::Heading => &self.heading,
      Fragment::Row => &self.row,
      Fragment::Edit => &self.edit,
      Fragment::About => &self.about,
    }
  }
}

const SELECTION_SENDS: Sends = Sends {
  variables: &[SentVariable {
    name: "offset",
    sent_as: VariableType::Vec2,
    values: &[],
    about: "How far it's moved from the first row to the selected one",
  }],
};

const HEADING_SENDS: Sends = Sends {
  variables: &[SentVariable {
    name: "text",
    sent_as: VariableType::Text,
    values: &[],
    about: "The heading",
  }],
};

const ROW_SENDS: Sends = Sends {
  variables: &[
    SentVariable {
      name: "label",
      sent_as: VariableType::Text,
      values: &[],
      about: "What the row is",
    },
    SentVariable {
      name: "value",
      sent_as: VariableType::Text,
      values: &[],
      about: "A setting's value, or no text",
    },
    SentVariable {
      name: "opens",
      sent_as: VariableType::Bool,
      values: &[],
      about: "Whether it opens a section, screen, or action",
    },
    SentVariable {
      name: "has_value",
      sent_as: VariableType::Bool,
      values: &[],
      about: "Whether it's a setting",
    },
    SentVariable {
      name: "changed",
      sent_as: VariableType::Bool,
      values: &[],
      about: "Whether it's a setting that isn't at its default",
    },
  ],
};

const EDIT_SENDS: Sends = Sends {
  variables: &[
    SentVariable {
      name: "label",
      sent_as: VariableType::Text,
      values: &[],
      about: "The setting's name",
    },
    SentVariable {
      name: "description",
      sent_as: VariableType::Text,
      values: &[],
      about: "What the setting does",
    },
    SentVariable {
      name: "value",
      sent_as: VariableType::Text,
      values: &[],
      about: "The setting's value",
    },
    SentVariable {
      name: "changed",
      sent_as: VariableType::Bool,
      values: &[],
      about: "Whether it isn't at its default",
    },
  ],
};

const ABOUT_SENDS: Sends = Sends {
  variables: &[SentVariable {
    name: "version",
    sent_as: VariableType::Text,
    values: &[],
    about: "Frontbox's version",
  }],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fragment {
  Frame,
  Selection,
  Heading,
  Row,
  Edit,
  About,
}

/// One part of what the menu shows: a fragment on a row, and the variables to set on it
#[derive(Debug, Clone, PartialEq)]
struct Part {
  fragment: Fragment,
  row: usize,
  variables: Vec<(String, Value)>,
  /// Eased to their values when the part is already shown, rather than set
  eased: Vec<(String, Value)>,
}

impl Part {
  fn new(fragment: Fragment, row: usize) -> Self {
    Self {
      fragment,
      row,
      variables: Vec::new(),
      eased: Vec::new(),
    }
  }

  fn variable(mut self, name: &str, value: impl Into<Value>) -> Self {
    self.variables.push((name.into(), value.into()));
    self
  }

  fn eased(mut self, name: &str, value: impl Into<Value>) -> Self {
    self.eased.push((name.into(), value.into()));
    self
  }

  /// The variables of a `row` fragment
  fn row(self, label: &str, value: &str, opens: bool, changed: bool) -> Self {
    self
      .variable("label", label)
      .variable("value", value)
      .variable("opens", opens)
      .variable("has_value", !opens)
      .variable("changed", changed)
  }

  /// Points each of `scene`'s variables whose default is a `selected_…` or `normal_…` value at the one for `selected`
  fn selected(mut self, scene: &Scene, selected: bool) -> Self {
    let state = if selected { "selected_" } else { "normal_" };
    for (name, default) in &scene.variables {
      let Value::Named(value) = default else {
        continue;
      };
      if let Some(rest) = value
        .strip_prefix("selected_")
        .or_else(|| value.strip_prefix("normal_"))
      {
        self
          .variables
          .push((name.clone(), Value::named(format!("{state}{rest}"))));
      }
    }
    self
  }
}

/// A part on the display
struct ShownPart {
  fragment: Fragment,
  row: usize,
  player: ScenePlayer,
}

#[cfg(test)]
mod tests {
  use frontbox_scene::{PlaybackId, SceneSystem};

  use super::*;

  const SWITCHES: MenuSwitches = MenuSwitches {
    coin_door: "door",
    back_btn: "back",
    select_btn: "select",
    inc_btn: "inc",
    dec_btn: "dec",
  };

  fn menu() -> DmdMenuSystem {
    let root = MenuSection::root()
      .section(MenuSection::new("Audio"))
      .section(MenuSection::new("Lights"));
    DmdMenuSystem::new(SWITCHES, Box::leak(Box::new(root)))
  }

  fn spawned(context: &mut TestContext) -> DmdMenuSystem {
    context.insert_system(SceneSystem::new());
    let mut menu = menu();
    menu.on_spawn(&context.sys_ctx());
    menu.on_tick(Duration::ZERO, &context.sys_ctx());
    menu
  }

  fn shown(context: &TestContext) -> Vec<PlaybackId> {
    let ctx = context.sys_ctx();
    let scenes = ctx.systems.get::<SceneSystem>().unwrap();
    scenes.shown().map(|(id, _)| id).collect()
  }

  fn variable<'a>(part: &'a Part, name: &str) -> Option<&'a Value> {
    part
      .variables
      .iter()
      .find(|(variable, _)| variable == name)
      .map(|(_, value)| value)
  }

  #[test]
  fn the_menu_is_its_frame_then_a_part_for_each_row() {
    let mut context = TestContext::default();
    let menu = spawned(&mut context);
    let parts = menu.parts();
    let fragments: Vec<_> = parts.iter().map(|part| (part.fragment, part.row)).collect();
    assert_eq!(
      fragments,
      [
        (Fragment::Frame, 0),
        (Fragment::Selection, 0),
        (Fragment::Heading, 0),
        (Fragment::Row, 1),
        (Fragment::Row, 2),
        (Fragment::Row, 3)
      ]
    );
    assert_eq!(variable(&parts[2], "text"), Some(&Value::from("Frontbox")));
    assert_eq!(variable(&parts[3], "label"), Some(&Value::from("Audio")));
    assert_eq!(variable(&parts[3], "opens"), Some(&Value::from(true)));
    assert_eq!(shown(&context).len(), 6);
  }

  #[test]
  fn the_selected_row_points_its_variables_at_the_selected_values() {
    let mut context = TestContext::default();
    let menu = spawned(&mut context);
    let parts = menu.parts();
    assert_eq!(
      variable(&parts[3], "text_color"),
      Some(&Value::named("selected_text_color"))
    );
    assert_eq!(
      variable(&parts[4], "text_color"),
      Some(&Value::named("normal_text_color"))
    );
  }

  #[test]
  fn moving_the_selection_keeps_the_playbacks_of_parts_in_the_same_place() {
    let mut context = TestContext::default();
    let mut menu = spawned(&mut context);
    let before = shown(&context);
    menu.navigate_inc(&context.sys_ctx());
    menu.on_tick(Duration::ZERO, &context.sys_ctx());
    assert_eq!(shown(&context), before);
    assert_eq!(
      variable(&menu.parts()[4], "text_color"),
      Some(&Value::named("selected_text_color"))
    );
  }

  #[test]
  fn the_selection_slides_to_the_selected_row() {
    let mut context = TestContext::default();
    let mut menu = spawned(&mut context);
    menu.navigate_inc(&context.sys_ctx());
    menu.on_tick(Duration::ZERO, &context.sys_ctx());
    let offset = |context: &TestContext| {
      let ctx = context.sys_ctx();
      let scenes = ctx.systems.get::<SceneSystem>().unwrap();
      let (_, selection) = scenes.shown().nth(1).unwrap();
      selection.moment().variables.get("offset").cloned()
    };
    // it starts on the first selectable row, below the heading
    let advance = |context: &TestContext, ms| {
      let ctx = context.sys_ctx();
      ctx
        .systems
        .with::<SceneSystem, _>(|scenes| scenes.on_tick(Duration::from_millis(ms), &ctx));
    };
    advance(&context, 60);
    let Some(Value::Vec2(halfway)) = offset(&context) else {
      panic!("no offset");
    };
    assert!(halfway.y > 7.0 && halfway.y < 14.0, "{halfway}");
    advance(&context, 100);
    assert_eq!(offset(&context), Some(Value::Vec2(Vec2::new(0.0, 14.0))));
  }

  #[test]
  fn a_screen_of_its_own_takes_the_place_of_the_rows() {
    let mut context = TestContext::default();
    let mut menu = spawned(&mut context);
    // About is last, after the two sections
    menu.navigate_inc(&context.sys_ctx());
    menu.navigate_inc(&context.sys_ctx());
    menu.navigate_fwd(&context.sys_ctx());
    menu.on_tick(Duration::ZERO, &context.sys_ctx());
    let parts = menu.parts();
    assert_eq!(
      parts.iter().map(|part| part.fragment).collect::<Vec<_>>(),
      [Fragment::About]
    );
    assert_eq!(shown(&context).len(), 1);
  }

  #[test]
  fn the_bundled_fragments_fit_what_the_menu_sends() {
    let content = content_with_bundled(None);
    let fragments = Fragments::open(&content, "frontbox/128x32/menu").unwrap();
    for (scene, sends) in [
      (&fragments.selection, SELECTION_SENDS),
      (&fragments.heading, HEADING_SENDS),
      (&fragments.row, ROW_SENDS),
      (&fragments.edit, EDIT_SENDS),
      (&fragments.about, ABOUT_SENDS),
    ] {
      assert_eq!(sends.check(scene), []);
    }
  }

  #[test]
  fn a_copy_is_moved_down_to_its_row_and_drawn_at_the_menus_z() {
    let content = content_with_bundled(None);
    let fragments = Fragments::open(&content, "frontbox/128x32/menu").unwrap();
    let copy = placed(&fragments.row, 2, 5);
    let layer = copy.layers[0].as_display().unwrap();
    assert_eq!(layer.z, 5);
    let translate = layer
      .layer
      .region
      .as_ref()
      .unwrap()
      .transform
      .last()
      .unwrap();
    assert!(
      matches!(translate, Transform::Translate(Prop::Fixed(offset)) if *offset == Vec2::new(0.0, 14.0))
    );
  }
}
