use std::collections::HashMap;

use frontbox::prelude::*;
use frontbox_canvas::*;

use crate::menu::{DmdMenuTheme, MenuSection};
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
  section_lookup: HashMap<u64, SectionEntry>,
  active_section: &'static MenuSection,
  rows: Vec<MenuRow>,
  selected_row: usize,
  selected_config: Option<Box<dyn GeneralizedConfigValue>>,
  // indicates that the rows vector needs to be re-generated because something changed
  requires_row_refresh: bool,
}

impl DmdMenuSystem {
  pub fn new(switch_names: MenuSwitches, root: &'static MenuSection, theme: DmdMenuTheme) -> Self {
    let mut section_lookup = HashMap::<u64, SectionEntry>::new();
    section_lookup.insert(
      root.id,
      SectionEntry {
        section: root,
        parent: None,
      },
    );
    Self::build_section_lookup(&mut section_lookup, root);

    Self {
      switch_names,
      theme,
      section_lookup,
      requires_row_refresh: true,
      active_section: root,
      rows: Vec::new(),
      selected_row: 0,
      selected_config: None,
    }
  }

  /// Build a quick id => section, section parent lookup table for fast navigation
  fn build_section_lookup(lookup: &mut HashMap<u64, SectionEntry>, parent: &'static MenuSection) {
    for section in &parent.sections {
      lookup.insert(
        section.id,
        SectionEntry {
          section,
          parent: Some(parent),
        },
      );
      Self::build_section_lookup(lookup, section);
    }
  }

  fn activate_section(&mut self, section: &'static MenuSection, ctx: &Context) {
    self.active_section = section;
    self.selected_row = 0;
    self.refresh_current_selection(ctx);
  }

  fn navigate_back(&mut self, ctx: &Context) {
    let parent = self
      .section_lookup
      .get(&self.active_section.id)
      .and_then(|e| e.parent);
    if let Some(parent) = parent {
      self.play_sound(BACK_SND, ctx);
      self.activate_section(parent, ctx);
    } else {
      self.play_sound(NOT_ALLOWED_SND, ctx);
    }
  }

  fn navigate_fwd(&mut self, ctx: &Context) {
    match self.rows[self.selected_row] {
      MenuRow::Section { id, .. } => {
        let section = self.section_lookup.get(&id).unwrap().section;
        self.activate_section(section, ctx);
      }
      MenuRow::Config { name, .. } => {
        // How to associate a menu row back to a config value?
        todo!();
      }
    }
  }

  // MenuRows contained memoized values which may change. Upon trigger, re-generate these rows with the latest values
  fn refresh_current_selection(&self, ctx: &Context) -> Vec<MenuRow> {
    let mut rows: Vec<MenuRow> = Vec::new();

    for section in &self.active_section.sections {
      rows.push(MenuRow::Section {
        id: section.id,
        name: section.name,
        selected: rows.len() == self.selected_row,
      });
    }

    for config in &self.active_section.configs {
      rows.push(MenuRow::Config {
        name: config.text(),
        is_default: !config.value_modified(ctx),
        value: config.current_value(ctx),
        selected: rows.len() == self.selected_row,
      });
    }

    rows
  }

  fn draw(&self, viewport: &Size<u32>) -> Container {
    let mut frame = Container::new(self.theme.menu_bg.clone()).with_padding(1, 1, 1, 1);
    if let Some(theme_border) = &self.theme.menu_border {
      if let Some(border) = frame.border_mut() {
        *border = theme_border.clone();
      }
    }

    let row_height = SIGI_REGULAR_5PX_FONT.height + 1;
    let mut acc_height = 0;
    let mut row_index = self
      .selected_row
      .saturating_sub(2)
      .clamp(0, self.rows.len());

    loop {
      if let Some(row) = self.rows.get(row_index) {
        let layer = match row {
          MenuRow::Section { name, selected, .. } => self.draw_section(name, *selected),
          MenuRow::Config {
            name,
            value,
            is_default,
            selected,
          } => self.draw_config(name, value.clone(), *is_default, *selected),
        };
        acc_height += row_height as u32;
        frame.add(layer.default_position());

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

  fn draw_section(&self, name: &'static str, selected: bool) -> Container {
    let bg: Fill2d = if selected {
      self.theme.selected_section_bg.clone()
    } else {
      self.theme.unselected_section_bg.clone()
    };
    let mut row = Container::new(bg).with_padding(1, 1, 1, 1);

    let text_color = if selected {
      self.theme.selected_section_color
    } else {
      self.theme.unselected_section_color
    };

    row.add(
      SIGI_REGULAR_5PX_FONT
        .overflow_text(name, text_color, 1)
        .default_position(),
    );
    row.add(
      SYMBOLS_5PX_REGULAR
        .text("▶", text_color, 1)
        .horizontal(Horizontal::RightOffset(Extent::full())),
    );

    row
  }

  fn draw_config(
    &self,
    name: &'static str,
    value: String,
    is_default: bool,
    selected: bool,
  ) -> Container {
    let bg: Fill2d = if selected {
      self.theme.selected_config_bg.clone()
    } else {
      self.theme.unselected_config_bg.clone()
    };

    let mut row = Container::new(bg).with_padding(1, 1, 1, 1);

    let text_color = if selected {
      self.theme.selected_config_color
    } else {
      self.theme.unselected_config_color
    };

    row.add(
      SIGI_REGULAR_5PX_FONT
        .overflow_text(name, text_color, 1)
        .default_position(),
    );

    let value_font = if is_default {
      &SIGI_REGULAR_5PX_FONT
    } else {
      &SIGI_BOLD_5PX_FONT
    };

    row.add(
      value_font
        .overflow_text(value, text_color, 1)
        .horizontal(Horizontal::right_offset(0)),
    );

    row
  }

  fn play_sound(&self, sound: &'static str, ctx: &Context) {
    // TODO
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
    // TODO: register sounds
    // TODO: need a direct dependency on frontbox-sound (is there a way to GameManager interface this?)
    self.on_reactivate(ctx);
  }

  fn on_reactivate(&mut self, ctx: &Context) {
    if let Some(mut dmd) = ctx.systems.get::<DmdSystem>() {
      dmd.clear();
      self.requires_row_refresh = true;
    }
    self.play_sound(MENU_BEGIN_SND, ctx);
  }

  fn on_tick(&mut self, _delta: Duration, ctx: &Context) {
    if self.requires_row_refresh {
      self.refresh_current_selection(ctx);
      self.requires_row_refresh = false;
    }

    // TODO: this should probably only update the layer if something has changed
    if let Some(mut dmd) = ctx.systems.get::<DmdSystem>() {
      let size = dmd.size().clone();
      dmd.insert_layer(0, self.draw(&size).default_position());
    }
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
    if let Some(event) = event.downcast_ref::<SwitchClosed>() {
      if event.switch.name == self.switch_names.back_btn {
        self.navigate_back(ctx);
      } else if event.switch.name == self.switch_names.select_btn {
        self.navigate_fwd(ctx);
      } else if event.switch.name == self.switch_names.inc_btn {
        todo!();
      } else if event.switch.name == self.switch_names.dec_btn {
        todo!();
      }
    }
  }
}

enum MenuRow {
  Section {
    id: u64,
    name: &'static str,
    selected: bool,
  },
  Config {
    name: &'static str,
    value: String,
    is_default: bool,
    selected: bool,
  },
}

pub struct MenuSwitches {
  pub coin_door: &'static str,
  pub back_btn: &'static str,
  pub select_btn: &'static str,
  pub inc_btn: &'static str,
  pub dec_btn: &'static str,
}

struct SectionEntry {
  section: &'static MenuSection,
  parent: Option<&'static MenuSection>,
}
