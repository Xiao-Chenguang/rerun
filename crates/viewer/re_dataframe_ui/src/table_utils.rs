use ahash::HashSet;
use egui::{Color32, Context, Frame, Id, RichText, Stroke, Style};
use re_ui::{UiExt as _, design_tokens_of, icons};

/// This applies some fixes so that the column resize bar is correctly displayed.
///
/// TODO(lucasmerlin): this might affect widgets within the table, and should probably be reverted
/// within the cell. Also should be properly fixed via `egui_table`.
pub fn apply_table_style_fixes(style: &mut Style) {
    let theme = if style.visuals.dark_mode {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };

    let design_tokens = design_tokens_of(theme);

    style.visuals.widgets.hovered.bg_stroke =
        Stroke::new(1.0, design_tokens.table_interaction_hovered_bg_stroke);
    style.visuals.widgets.active.bg_stroke =
        Stroke::new(1.0, design_tokens.table_interaction_active_bg_stroke);
    // regular vertical lines are drawn in cell_ui to allow cells to be connected
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(0.0, Color32::TRANSPARENT);
}

pub fn header_title(
    ui: &mut egui::Ui,
    table_style: re_ui::TableStyle,
    title: impl Into<RichText>,
) -> egui::Response {
    header_ui(ui, table_style, false, |ui| {
        ui.monospace(title.into().strong());
    })
    .response
}

pub fn header_ui<R>(
    ui: &mut egui::Ui,
    table_style: re_ui::TableStyle,
    connected_to_next_cell: bool,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let rect = ui.max_rect();
    ui.painter()
        .rect_filled(rect, 0.0, ui.tokens().table_header_bg_fill);

    let response = Frame::new()
        .inner_margin(ui.tokens().header_cell_margin(table_style))
        .show(ui, content);

    if !connected_to_next_cell {
        ui.painter().vline(
            rect.max.x - 1.0,
            rect.y_range(),
            Stroke::new(1.0, ui.tokens().table_header_stroke_color),
        );
    }

    ui.painter().hline(
        rect.x_range(),
        rect.max.y - 1.0, // - 1.0 prevents it from being overdrawn by the following row
        Stroke::new(1.0, ui.tokens().table_header_stroke_color),
    );

    response
}

pub fn cell_ui<R>(
    ui: &mut egui::Ui,
    table_style: re_ui::TableStyle,
    connected_to_next_cell: bool,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let response = Frame::new()
        .inner_margin(ui.tokens().table_cell_margin(table_style))
        .show(ui, content);

    let rect = ui.max_rect();

    if !connected_to_next_cell {
        ui.painter().vline(
            rect.max.x - 1.0,
            rect.y_range(),
            Stroke::new(1.0, ui.tokens().table_interaction_noninteractive_bg_stroke),
        );
    }

    ui.painter().hline(
        rect.x_range(),
        rect.max.y - 1.0, // - 1.0 prevents it from being overdrawn by the following row
        Stroke::new(1.0, ui.tokens().table_interaction_noninteractive_bg_stroke),
    );

    response
}

#[derive(Debug, Clone, Hash, serde::Serialize, serde::Deserialize)]
pub struct ColumnConfig {
    id: Id,
    name: String,
    visible: bool,
}

impl ColumnConfig {
    pub fn new(id: Id, name: String) -> Self {
        Self {
            id,
            name,
            visible: true,
        }
    }

    pub fn new_with_visible(id: Id, name: String, visible: bool) -> Self {
        Self { id, name, visible }
    }

    pub fn id(&self) -> Id {
        self.id
    }
}

// TODO(lucasmerlin): It would be nice to have this in egui_table, so egui_table could do the work
// of showing / hiding columns based on the config.
// https://github.com/rerun-io/egui_table/issues/27
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TableConfig {
    id: Id,
    columns: Vec<ColumnConfig>,
}

impl TableConfig {
    fn new(id: Id) -> Self {
        Self {
            id,
            columns: Vec::new(),
        }
    }

    /// Remove the table config from the cache.
    pub fn clear_state(ctx: &Context, persisted_id: Id) {
        ctx.data_mut(|data| {
            data.remove::<Self>(persisted_id);
        });
    }

    /// Get a table config, creating it if it doesn't exist.
    ///
    /// Columns is an iterator of default [`ColumnConfig`]s that will be added to the table config.
    /// Any columns that are not in the iterator will be removed from the table config.
    /// New columns will be added in order at the end.
    ///
    /// Don't forget to call [`Self::store`] to persist the changes.
    pub fn get_with_columns(
        ctx: &Context,
        persisted_id: Id,
        columns: impl Iterator<Item = ColumnConfig>,
    ) -> Self {
        ctx.data_mut(|data| {
            let config: &mut Self =
                data.get_persisted_mut_or_insert_with(persisted_id, || Self::new(persisted_id));

            let mut has_cols = HashSet::default();

            for col in columns {
                has_cols.insert(col.id);
                if !config.columns.iter().any(|c| c.name == col.name) {
                    config.columns.push(col);
                }
            }

            config.columns.retain(|col| has_cols.contains(&col.id));

            config.clone()
        })
    }

    pub fn store(self, ctx: &Context) {
        ctx.data_mut(|data| {
            data.insert_persisted(self.id, self);
        });
    }

    pub fn visible_columns(&self) -> impl Iterator<Item = &ColumnConfig> {
        self.columns.iter().filter(|col| col.visible)
    }

    pub fn visible_column_names(&self) -> impl Iterator<Item = &str> {
        self.visible_columns().map(|col| col.name.as_str())
    }

    pub fn visible_column_ids(&self) -> impl Iterator<Item = Id> + use<'_> {
        self.visible_columns().map(|col| col.id)
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let response = egui_dnd::dnd(ui, "Columns").show(
            self.columns.iter_mut(),
            |ui, column, handle, _state| {
                let visible = column.visible;
                egui::Sides::new().show(
                    ui,
                    |ui| {
                        handle.ui(ui, |ui| {
                            ui.small_icon(&icons::DND_HANDLE, None);
                        });
                        let mut label = RichText::new(&column.name);
                        if visible {
                            label = label.strong();
                        } else {
                            label = label.weak();
                        }
                        ui.label(label);
                    },
                    |ui| {
                        let (icon, alt_text) = if column.visible {
                            (&icons::VISIBLE, "Hide column")
                        } else {
                            (&icons::INVISIBLE, "Show column")
                        };
                        if ui.small_icon_button(icon, alt_text).clicked() {
                            column.visible = !column.visible;
                        }
                    },
                );
            },
        );
        if response.is_drag_finished() {
            response.update_vec(self.columns.as_mut_slice());
        }
    }

    pub fn button_ui(&mut self, ui: &mut egui::Ui) {
        ui.menu_image_text_button(icons::SETTINGS.as_image(), "Columns", |ui| {
            self.ui(ui);
        });
    }
}
