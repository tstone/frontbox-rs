use std::collections::HashMap;

use frontbox::prelude::*;
use frontbox_canvas::*;
#[cfg(feature = "sound")]
use frontbox_sound::*;

use crate::menu::{DmdMenuTheme, MenuRow, MenuSection};
use crate::*;

const SELECT_SND: &'static str = "dmd_menu_select";
const INC_SOUND: &'static str = "dmd_menu_inc";
const DEC_SOUND: &'static str = "dmd_menu_dec";
const BACK_SND: &'static str = "dmd_menu_back";
const NOT_ALLOWED_SND: &'static str = "dmd_menu_not_allowed";
const MENU_BEGIN_SND: &'static str = "dmd_menu_begin";
const MENU_END_SND: &'static str = "dmd_menu_end";

pub struct DmdMenuSystem {
  switch_names: MenuSwitches,
  theme: DmdMenuTheme,
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
  pub fn new(switch_names: MenuSwitches, root: &'static MenuSection, theme: DmdMenuTheme) -> Self {
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
    Self::build_row_lookup(&mut row_lookup, &root);

    Self {
      switch_names,
      theme,
      row_lookup,
      active_section: &root,
      display_rows: Vec::new(),
      selected_row: Self::first_non_header_row(&root),
      selected_config: None,
      selected_special: None,
      requires_row_refresh: true,
      requires_render: true,
    }
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
      .position(|r| match r {
        MenuRow::Heading(_, _) => false,
        _ => true,
      })
      .unwrap_or(0)
  }

  fn activate_section(&mut self, section: &'static MenuSection, ctx: &Context) {
    self.active_section = section;
    self.selected_row = Self::first_non_header_row(self.active_section);
    self.refresh_display_rows(ctx);
  }

  fn navigate_fwd(&mut self, ctx: &Context) {
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
      }
      _ => {}
    }
  }

  fn navigate_back(&mut self, ctx: &Context) {
    if let Some(_) = &self.selected_config {
      self.selected_config = None;
      self.requires_render = true;
      ctx.play_sfx(BACK_SND);
    } else if let Some(_) = &self.selected_special {
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

  fn navigate_inc(&mut self, ctx: &Context) {
    if let Some(row) = &mut self.selected_config {
      self
        .display_rows
        .iter_mut()
        .find(|r| r.id() == row.id())
        .unwrap()
        .value_mut()
        .map(|v| {
          *v = row.increment(ctx);
          log::debug!("increment: {}", v);
        });
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
  }

  fn navigate_dec(&mut self, ctx: &Context) {
    if let Some(row) = &self.selected_config {
      self
        .display_rows
        .iter_mut()
        .find(|r| r.id() == row.id())
        .unwrap()
        .value_mut()
        .map(|v| *v = row.decrement(ctx));
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
  }

  // Display rows contained memoized values which may change. Upon trigger, re-generate these rows with the latest values
  fn refresh_display_rows(&mut self, ctx: &Context) {
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
          self.display_rows.push(DisplayMenuRow::Config {
            id: *id,
            name: config.text(),
            desc: config.description(),
            is_default: !config.value_modified(ctx),
            value: config.current_value(ctx),
          });
        }
        MenuRow::Heading(id, text) => {
          self.display_rows.push(DisplayMenuRow::Heading {
            id: *id,
            text: text.to_string(),
          });
        }
        MenuRow::Special(_, text) => self.display_rows.push(DisplayMenuRow::Special(text)),
      }
    }

    self.requires_render = true;
  }

  fn draw(&self, viewport: &Size<u32>) -> Container {
    if let Some(row) = self.selected_config {
      if let Some(DisplayMenuRow::Config {
        name, desc, value, ..
      }) = self.display_rows.iter().find(|r| r.id() == row.id())
      {
        return self.draw_config_edit(name, desc, value.clone());
      }
    } else if let Some(special) = self.selected_special {
      match special {
        "About" => {
          return self.draw_special_about();
        }
        _ => todo!("Special menu screen not implemented: {}", special),
      }
    }

    self.draw_menu(viewport)
  }

  fn draw_menu(&self, viewport: &Size<u32>) -> Container {
    let mut frame = Container::new(self.theme.menu_bg.clone());
    if let Some(theme_border) = &self.theme.menu_border {
      if let Some(border) = frame.border_mut() {
        *border = theme_border.clone();
        frame.padding = Padding::new(1, 1, 1, 1);
      }
    }

    let row_height = SIGI_REGULAR_5PX_FONT.height as u32 + 2; // 2 leading between
    let mut acc_height = 0;
    let mut row_index = self
      .selected_row
      .saturating_sub(2)
      .clamp(0, self.display_rows.len());

    // draw rows that fit into the viewport
    loop {
      if let Some(row) = self.display_rows.get(row_index) {
        let selected = row_index == self.selected_row;
        let layer = match row {
          DisplayMenuRow::Section { name, .. } => self.draw_section(name, selected),
          DisplayMenuRow::Config {
            #[allow(unused)]
            id,
            name,
            value,
            is_default,
            ..
          } => self.draw_config_row(name, value.clone(), *is_default, selected),
          DisplayMenuRow::Heading { text, .. } => self.draw_heading(text),
          DisplayMenuRow::Special(name) => self.draw_section(name, selected),
        };

        frame.add(
          layer
            .vertical(Vertical::top_offset(acc_height))
            .height(row_height),
        );
        acc_height += row_height as u32;

        if acc_height >= viewport.height {
          break;
        }
        row_index += 1;
      } else {
        break;
      }
    }

    frame
  }

  fn draw_section(&self, name: &str, selected: bool) -> Container {
    let bg: Fill2d = if selected {
      self.theme.selected_bg.clone()
    } else {
      self.theme.unselected_bg.clone()
    };

    let mut row = Container::new(bg).with_padding(1, 1, 0, 0);

    let text_color = if selected {
      self.theme.selected_color
    } else {
      self.theme.unselected_color
    };

    row.add(
      SIGI_REGULAR_5PX_FONT
        .overflow_text(name, text_color, 1)
        .default_position(),
    );

    row.add(
      SYMBOLS_5PX_REGULAR_FONT
        .text("▶", text_color, 1)
        .width(5)
        .horizontal(Horizontal::right_offset(2)),
    );

    row
  }

  fn draw_config_row(
    &self,
    name: &'static str,
    value: String,
    is_default: bool,
    selected: bool,
  ) -> Container {
    let bg: Fill2d = if selected {
      self.theme.selected_bg.clone()
    } else {
      self.theme.unselected_bg.clone()
    };

    let mut row = Container::new(bg).with_padding(1, 1, 0, 0);

    let text_color = if selected {
      self.theme.selected_color
    } else {
      self.theme.unselected_color
    };

    row.add(
      SIGI_CONDENSED_REGULAR_5PX_FONT
        .overflow_text(name, text_color, 1)
        .width(0.70),
    );

    let value_font = if is_default {
      &SIGI_CONDENSED_REGULAR_5PX_FONT
    } else {
      &SIGI_CONDENSED_BOLD_5PX_FONT
    };

    row.add(
      value_font
        .overflow_text(value, text_color, 1)
        .width(0.27)
        .horizontal(Horizontal::right_offset(0)),
    );

    row
  }

  fn draw_heading(&self, text: &str) -> Container {
    let mut row = Container::new(self.theme.heading_bg.clone()).with_padding(1, 1, 0, 0);
    row.add(
      SIGI_REGULAR_5PX_FONT
        .overflow_text(text, self.theme.heading_color, 1)
        .default_position(),
    );
    row
  }

  fn draw_config_edit(&self, name: &'static str, desc: &'static str, value: String) -> Container {
    let mut window = Container::new(self.theme.menu_bg.clone()).with_padding_all(2);

    // TODO: need a text_block which handles multi-line text

    window.add(
      SIGI_REGULAR_5PX_FONT
        .text(name, self.theme.heading_color, 1)
        .default_position(),
    );

    window.add(
      SIGI_CONDENSED_REGULAR_5PX_FONT
        .text(desc, self.theme.unselected_color, 1)
        .width(0.70)
        .top_offset(9),
    );

    window.add(
      SIGI_REGULAR_5PX_FONT
        .text(value, self.theme.unselected_color, 1)
        .top_offset(9)
        .right_offset(0)
        .width(0.25),
    );

    window
  }

  fn draw_special_about(&self) -> Container {
    // TODO: some kind of nice graphic here instead of this placeholder
    let mut window = Container::new(Fill2d::Gradient(
      vec![
        GradientStop::new(0.0, Rgba([0xf8, 0xf8, 0xf2, 0xff]).darken(0.15)),
        GradientStop::new(0.25, Rgba::cyan()),
        GradientStop::new(1.0, Rgba([0xf9, 0x26, 0x72, 0xff])),
      ],
      45.0,
    ));
    window.add(
      SIGI_BOLD_7PX_FONT
        .text("Frontbox", Rgba::black(), 1)
        .width(63)
        .horizontal(Horizontal::Centered)
        .top_offset(6),
    );
    window.add(
      SIGI_CONDENSED_BOLD_5PX_FONT
        .text(env!("CARGO_PKG_VERSION"), Rgba::black(), 1)
        .width(25)
        .horizontal(Horizontal::Centered)
        .top_offset(15),
    );

    window.add(
      SIGI_CONDENSED_REGULAR_5PX_FONT
        .text("Neon Blue Pinball 2026", Rgba::black(), 1)
        .width(102)
        .horizontal(Horizontal::Centered)
        .top_offset(23),
    );
    window
  }
}

impl System for DmdMenuSystem {
  fn is_active(&self, ctx: &Context) -> bool {
    ctx
      .switches
      .is_open(self.switch_names.coin_door)
      .unwrap_or(false)
  }

  fn on_spawn(&mut self, ctx: &Context) {
    let exe_dir = std::env::current_exe()
      .unwrap()
      .parent()
      .unwrap()
      .to_path_buf();
    ctx.preload_sound(MENU_BEGIN_SND, exe_dir.join("assets/sounds/door-open.mp3"));

    // TODO: register all sounds

    self.on_reactivate(ctx);
  }

  fn on_reactivate(&mut self, ctx: &Context) {
    if let Some(mut dmd) = ctx.systems.get::<DmdSystem>() {
      dmd.clear();
      self.requires_row_refresh = true;
    }
    ctx.play_sfx(MENU_BEGIN_SND);
  }

  fn on_deactivate(&mut self, ctx: &Context) {
    ctx.play_sfx(MENU_END_SND);
  }

  fn on_tick(&mut self, _delta: Duration, ctx: &Context) {
    if self.requires_row_refresh {
      self.refresh_display_rows(ctx);
      self.requires_row_refresh = false;
    }

    if self.requires_render
      && let Some(mut dmd) = ctx.systems.get::<DmdSystem>()
    {
      let size = dmd.size().clone();
      dmd.insert_layer(0, self.draw(&size).default_position());
    }
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
    if let Some(event) = event.downcast_ref::<SwitchClosed>() {
      log::debug!("Switch event");

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
    is_default: bool,
  },
  Heading {
    id: u64,
    text: String,
  },
  Special(&'static str),
}

impl DisplayMenuRow {
  pub fn id(&self) -> u64 {
    match self {
      DisplayMenuRow::Config { id, .. } => *id,
      DisplayMenuRow::Section { id, .. } => *id,
      DisplayMenuRow::Heading { id, .. } => *id,
      DisplayMenuRow::Special(_) => 0,
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
