use std::sync::Arc;

use frontbox::prelude::*;

use crate::menu::MenuSection;
use crate::{Fill, Frame, PixelFont, Renderable};

pub struct Menu {
  font: &'static PixelFont,
  rows: Vec<MenuRow>,
  selected_config: Option<Arc<&'static dyn GeneralizedConfigValue>>,
}

impl Menu {
  pub fn new(root: MenuSection, font: &'static PixelFont) -> Self {
    let mut rows = Self::section_to_rows(&root);
    // start with the first row selected
    if let Some(first) = rows.get_mut(0) {
      *first.highlighted_mut() = true;
    }

    Self {
      font,
      rows,
      selected_config: None,
    }
  }

  pub fn increment(&mut self) {
    if let Some(selected_config) = &self.selected_config {
      todo!();
    } else {
      if let Some((rows, highlighted)) = Self::find_current_highlight_mut(&mut self.rows) {
        let next = highlighted.saturating_add(1).clamp(0, rows.len());
        *rows[highlighted].highlighted_mut() = false;
        *rows[next].highlighted_mut() = true;
      }
    }
  }

  pub fn decrement(&mut self) {
    if let Some(selected_config) = &self.selected_config {
      todo!();
    } else {
      if let Some((rows, highlighted)) = Self::find_current_highlight_mut(&mut self.rows) {
        let prev = highlighted.saturating_sub(1).clamp(0, rows.len());
        *rows[highlighted].highlighted_mut() = false;
        *rows[prev].highlighted_mut() = true;
      }
    }
  }

  pub fn select(&mut self) {
    if let Some(selected_config) = &self.selected_config {
      self.selected_config = None;
    } else {
      if let Some((rows, highlighted)) = Self::find_current_highlight_mut(&mut self.rows) {
        *rows[highlighted].highlighted_mut() = false;
        match &mut rows[highlighted] {
          MenuRow::Section { rows, .. } => {
            // select first item of child
            if let Some(row) = rows.get_mut(0) {
              *row.highlighted_mut() = true;
            }
          }
          MenuRow::Config { config, .. } => self.selected_config = Some(config.clone()),
        }
      }
    }
  }

  pub fn back(&mut self) {}

  pub fn render(&self, frame: &mut Frame, ctx: &Context) {
    if let Some(selected_config) = &self.selected_config {
      // render a view with the description, default, and current value
      todo!();
    } else {
      self.render_menu_list(frame, ctx);
    }
  }

  fn render_menu_list(&self, frame: &mut Frame, ctx: &Context) {
    let row_height = (self.font.height() + 1) as usize;
    let total_rows_allowed = (frame.height() - 1) / row_height;
    let item_col_width = (frame.width() as f32 * 0.6) as usize;
    let mut item_col = Frame::new(item_col_width, frame.height(), Fill::Transparent);
    let mut value_col = Frame::new(
      frame.width() - item_col_width - 1,
      frame.height(),
      Fill::Transparent,
    );

    // find selection
    if let Some((rows, selected)) = Self::find_current_highlight(&self.rows) {
      let mut rendered_rows = 0;
      let mut row_offset = 0;

      let starting_index = selected
        .saturating_sub(total_rows_allowed)
        .clamp(0, rows.len());

      for i in starting_index..rows.len() {
        if let Some(row) = rows.get(i) {
          // render
          match row {
            MenuRow::Section {
              section,
              rows,
              highlighted,
            } => self.render_section(
              &mut item_col,
              &mut value_col,
              section,
              rows.len(),
              *highlighted,
              row_offset,
            ),
            MenuRow::Config {
              config,
              highlighted,
            } => self.render_config(
              &mut item_col,
              &mut value_col,
              config,
              *highlighted,
              row_offset,
              ctx,
            ),
          }

          // exit
          rendered_rows += 1;
          row_offset += row_height;
          if rendered_rows == total_rows_allowed {
            break;
          }
        }
      }
    }

    frame.add(item_col);
    frame.add(value_col.left(item_col_width as isize));
  }

  fn highlight_color(selected: bool) -> Rgba<u8> {
    if selected {
      Rgba::red()
    } else {
      Rgba::yellow()
    }
  }

  fn render_section(
    &self,
    item_column: &mut Frame,
    value_column: &mut Frame,
    section: &MenuSection,
    row_count: usize,
    highlighted: bool,
    row_offset: usize,
  ) {
    let color = Self::highlight_color(highlighted);

    item_column.add(
      self
        .font
        .text(section.name, color)
        .offset(0, row_offset as isize),
    );
    value_column.add(
      self
        .font
        .text(format!("({})", row_count), color)
        .offset(0, row_offset as isize),
    )
  }

  fn render_config(
    &self,
    item_column: &mut Frame,
    value_column: &mut Frame,
    config: &Arc<&'static dyn GeneralizedConfigValue>,
    highlighted: bool,
    row_offset: usize,
    ctx: &Context,
  ) {
    let color = Self::highlight_color(highlighted);

    item_column.add(
      self
        .font
        .text(config.text(), color)
        .offset(0, row_offset as isize),
    );
    value_column.add(
      self
        .font
        .text(config.current_value(ctx), color)
        .offset(0, row_offset as isize),
    )
  }

  fn find_current_highlight(rows: &Vec<MenuRow>) -> Option<(&Vec<MenuRow>, usize)> {
    for (i, row) in rows.iter().enumerate() {
      match row {
        MenuRow::Config { highlighted, .. } if *highlighted => return Some((rows, i)),
        MenuRow::Section { highlighted, .. } if *highlighted => return Some((rows, i)),
        MenuRow::Section { rows, .. } => match Self::find_current_highlight(rows) {
          Some(rs) => return Some(rs),
          None => {}
        },
        _ => {}
      }
    }

    None
  }

  fn find_current_highlight_mut(rows: &mut Vec<MenuRow>) -> Option<(&mut Vec<MenuRow>, usize)> {
    for (i, row) in rows.iter_mut().enumerate() {
      match row {
        MenuRow::Config { highlighted, .. } if *highlighted => {
          return Some((rows, i));
        }
        MenuRow::Section { highlighted, .. } if *highlighted => {
          return Some((rows, i));
        }
        _ => {}
      }
    }

    // second pass for recursion — see below
    for row in rows.iter_mut() {
      match row {
        // search child rows
        MenuRow::Section { rows, .. } => {
          if let Some(found) = Self::find_current_highlight_mut(rows) {
            return Some(found);
          }
        }
        _ => {}
      }
    }

    None
  }

  fn section_to_rows(section: &MenuSection) -> Vec<MenuRow> {
    let mut rows: Vec<MenuRow> = Vec::new();

    for section in &section.sections {
      let children = Self::section_to_rows(section);
      rows.push(MenuRow::Section {
        section: section.clone(),
        rows: children,
        highlighted: false,
      })
    }

    for config in &section.configs {
      rows.push(MenuRow::Config {
        config: config.clone(),
        highlighted: false,
      })
    }

    rows
  }
}

enum MenuRow {
  Section {
    section: MenuSection,
    rows: Vec<MenuRow>,
    highlighted: bool,
  },
  Config {
    config: Arc<&'static dyn GeneralizedConfigValue>,
    highlighted: bool,
  },
}

impl MenuRow {
  pub fn highlighted(&self) -> bool {
    match self {
      Self::Section {
        highlighted: selected,
        ..
      } => *selected,
      Self::Config {
        highlighted: selected,
        ..
      } => *selected,
    }
  }

  pub fn highlighted_mut(&mut self) -> &mut bool {
    match self {
      Self::Section {
        highlighted: selected,
        ..
      } => selected,
      Self::Config {
        highlighted: selected,
        ..
      } => selected,
    }
  }
}
