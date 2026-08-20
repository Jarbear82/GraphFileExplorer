use gpui::prelude::*;
use gpui::{
    Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, Render, Styled, Window,
    div, px,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::dock::{BasePanel, Panel, PanelEvent};
use gpui_component::menu::{ContextMenuExt, PopupMenu, PopupMenuItem};
use gpui_component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, StyledExt, h_flex, label::Label, v_flex,
};

use crate::model::fs_entry::format_bytes;
use crate::workspace::Workspace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Name,
    Size,
    Type,
    Modified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

pub struct TableView {
    workspace: Entity<Workspace>,
    focus_handle: FocusHandle,
    sort_field: SortField,
    sort_order: SortOrder,
}

impl TableView {
    pub fn new(workspace: Entity<Workspace>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            workspace,
            focus_handle: cx.focus_handle(),
            sort_field: SortField::Name,
            sort_order: SortOrder::Asc,
        }
    }

    pub fn set_sort(&mut self, field: SortField, cx: &mut Context<Self>) {
        if self.sort_field == field {
            self.sort_order = match self.sort_order {
                SortOrder::Asc => SortOrder::Desc,
                SortOrder::Desc => SortOrder::Asc,
            };
        } else {
            self.sort_field = field;
            self.sort_order = SortOrder::Asc;
        }
        cx.notify();
    }
}

impl EventEmitter<PanelEvent> for TableView {}

impl Focusable for TableView {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BasePanel for TableView {
    fn panel_name(&self) -> &'static str {
        "TableView"
    }
}

impl Panel for TableView {
    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        "Directory Table"
    }
}

impl Render for TableView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (current_entry, selected_path, current_path) = {
            let ws = self.workspace.read(cx);
            (
                ws.current_entry.clone(),
                ws.selected_path.clone(),
                ws.current_path.clone(),
            )
        };

        let mut entries = if let Some(entry) = current_entry {
            entry.children
        } else {
            Vec::new()
        };

        // Sort items according to sort_field & sort_order
        let field = self.sort_field;
        let order = self.sort_order;
        entries.sort_by(|a, b| {
            let cmp = match field {
                SortField::Name => b
                    .is_dir
                    .cmp(&a.is_dir)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
                SortField::Size => b
                    .is_dir
                    .cmp(&a.is_dir)
                    .then_with(|| a.size_bytes.cmp(&b.size_bytes)),
                SortField::Type => b
                    .is_dir
                    .cmp(&a.is_dir)
                    .then_with(|| a.category.display_badge().cmp(b.category.display_badge())),
                SortField::Modified => a.modified.cmp(&b.modified),
            };
            match order {
                SortOrder::Asc => cmp,
                SortOrder::Desc => cmp.reverse(),
            }
        });

        let ws = self.workspace.clone();

        let sort_label = |f: SortField, base: &str| {
            if field == f {
                match order {
                    SortOrder::Asc => format!("{base} ↑"),
                    SortOrder::Desc => format!("{base} ↓"),
                }
            } else {
                base.to_string()
            }
        };

        v_flex()
            .size_full()
            .overflow_hidden()
            .bg(cx.theme().background)
            // Table Toolbar Header
            .child(
                h_flex()
                    .w_full()
                    .h(px(36.0))
                    .px_3()
                    .py_1()
                    .gap_2()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().secondary.opacity(0.25))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(Icon::new(IconName::Frame).small())
                            .child(
                                Label::new(format!(
                                    "{} ({} items)",
                                    current_path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_else(|| "Root".to_string()),
                                    entries.len()
                                ))
                                .font_bold()
                                .text_sm(),
                            ),
                    ),
            )
            // Table Body
            .child(
                div()
                    .id("table-view-scroll-container")
                    .flex_1()
                    .min_h(px(0.0))
                    .size_full()
                    .overflow_y_scroll()
                    .p_2()
                    .child(
                        Table::new()
                            .child(
                                TableHeader::new().child(
                                    TableRow::new()
                                        .child(
                                            TableHead::new()
                                                .w(px(280.0))
                                                .child(
                                                    Button::new("sort-name")
                                                        .label(sort_label(SortField::Name, "Name"))
                                                        .ghost()
                                                        .on_click(cx.listener(|this, _event, _window, cx| {
                                                            this.set_sort(SortField::Name, cx);
                                                        })),
                                                ),
                                        )
                                        .child(
                                            TableHead::new()
                                                .w(px(110.0))
                                                .child(
                                                    Button::new("sort-size")
                                                        .label(sort_label(SortField::Size, "Size"))
                                                        .ghost()
                                                        .on_click(cx.listener(|this, _event, _window, cx| {
                                                            this.set_sort(SortField::Size, cx);
                                                        })),
                                                ),
                                        )
                                        .child(
                                            TableHead::new()
                                                .w(px(100.0))
                                                .child(
                                                    Button::new("sort-type")
                                                        .label(sort_label(SortField::Type, "Type"))
                                                        .ghost()
                                                        .on_click(cx.listener(|this, _event, _window, cx| {
                                                            this.set_sort(SortField::Type, cx);
                                                        })),
                                                ),
                                        )
                                        .child(
                                            TableHead::new()
                                                .w(px(140.0))
                                                .child(
                                                    Button::new("sort-modified")
                                                        .label(sort_label(SortField::Modified, "Modified"))
                                                        .ghost()
                                                        .on_click(cx.listener(|this, _event, _window, cx| {
                                                            this.set_sort(SortField::Modified, cx);
                                                        })),
                                                ),
                                        )
                                        .child(TableHead::new().w(px(80.0)).child("Action")),
                                ),
                            )
                            .child(
                                TableBody::new().children(entries.into_iter().map(|item| {
                                    let path = item.path.clone();
                                    let is_dir = item.is_dir;
                                    let is_sel = selected_path.as_ref() == Some(&path);
                                    let name = item.name.clone();
                                    let badge = item.category.display_badge();
                                    let size_str = if is_dir {
                                        format!("{} items", item.item_count)
                                    } else {
                                        format_bytes(item.size_bytes)
                                    };

                                    let mod_str = item
                                        .modified
                                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                        .map(|d| {
                                            let secs = d.as_secs();
                                            let hours = (secs / 3600) % 24;
                                            let mins = (secs / 60) % 60;
                                            format!("{:02}:{:02} UTC", hours, mins)
                                        })
                                        .unwrap_or_else(|| "—".to_string());

                                    let ws_click = ws.clone();
                                    let ws_drill = ws.clone();
                                    let p_click = path.clone();
                                    let p_drill = path.clone();

                                    let ws_ctx = ws.clone();
                                    let p_ctx = path.clone();

                                    let p_open_btn = path.clone();

                                    TableRow::new()
                                        .child(
                                            TableCell::new()
                                                .w(px(280.0))
                                                .child(
                                                    h_flex()
                                                        .id(format!("tbl-item-{}", path.display()))
                                                        .items_center()
                                                        .gap_2()
                                                        .cursor_pointer()
                                                        .on_click(move |_event, _window, cx| {
                                                            let p = p_click.clone();
                                                            ws_click.update(cx, |ws, cx| {
                                                                ws.select_path(Some(p), cx);
                                                            });
                                                        })
                                                        .context_menu(move |menu: PopupMenu, _window, _cx| {
                                                            let p_open = p_ctx.clone();
                                                            let p_reveal = p_ctx.clone();
                                                            let p_copy = p_ctx.clone();
                                                            let p_del = p_ctx.clone();
                                                            let ws_open = ws_ctx.clone();
                                                            let ws_del = ws_ctx.clone();

                                                            menu
                                                                .item(
                                                                    PopupMenuItem::new(if is_dir { "Drill Down" } else { "Open in Editor" })
                                                                        .icon(if is_dir { IconName::ChevronRight } else { IconName::ExternalLink })
                                                                        .on_click(move |_event, _window, cx| {
                                                                            let p = p_open.clone();
                                                                            if is_dir {
                                                                                ws_open.update(cx, |ws, cx| { ws.drill_down(p, cx); });
                                                                            } else {
                                                                                Workspace::open_in_system_editor(&p);
                                                                            }
                                                                        })
                                                                )
                                                                .item(
                                                                    PopupMenuItem::new("Reveal in File Manager")
                                                                        .icon(IconName::FolderOpen)
                                                                        .on_click(move |_event, _window, _cx| {
                                                                            Workspace::reveal_in_file_manager(&p_reveal);
                                                                        })
                                                                )
                                                                .item(
                                                                    PopupMenuItem::new("Copy Full Path")
                                                                        .icon(IconName::Copy)
                                                                        .on_click(move |_event, _window, cx| {
                                                                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(p_copy.to_string_lossy().to_string()));
                                                                        })
                                                                )
                                                                .separator()
                                                                .item(
                                                                    PopupMenuItem::new("Delete")
                                                                        .icon(IconName::Delete)
                                                                        .on_click(move |_event, _window, cx| {
                                                                            let p = p_del.clone();
                                                                            ws_del.update(cx, |ws, cx| {
                                                                                let _ = ws.delete_entry(&p, cx);
                                                                            });
                                                                        })
                                                                )
                                                        })
                                                        .child(Icon::new(if is_dir {
                                                            IconName::Folder
                                                        } else {
                                                            IconName::File
                                                        }).small())
                                                        .child(
                                                            Label::new(name)
                                                                .text_xs()
                                                                .font_semibold()
                                                                .text_color(if is_sel {
                                                                    cx.theme().primary
                                                                } else {
                                                                    cx.theme().foreground
                                                                }),
                                                        ),
                                                ),
                                        )
                                        .child(
                                            TableCell::new()
                                                .w(px(110.0))
                                                .child(Label::new(size_str).text_xs().text_color(cx.theme().muted_foreground)),
                                        )
                                        .child(
                                            TableCell::new().w(px(100.0)).child(
                                                div()
                                                    .px_1p5()
                                                    .py_0p5()
                                                    .rounded_sm()
                                                    .text_xs()
                                                    .font_bold()
                                                    .bg(if is_dir {
                                                        cx.theme().secondary
                                                    } else {
                                                        cx.theme().primary.opacity(0.15)
                                                    })
                                                    .text_color(if is_dir {
                                                        cx.theme().foreground
                                                    } else {
                                                        cx.theme().primary
                                                    })
                                                    .child(badge),
                                            ),
                                        )
                                        .child(
                                            TableCell::new()
                                                .w(px(140.0))
                                                .child(Label::new(mod_str).text_xs().text_color(cx.theme().muted_foreground)),
                                        )
                                        .child(
                                            TableCell::new().w(px(80.0)).child(
                                                if is_dir {
                                                    Button::new(format!("tbl-drill-{}", path.display()))
                                                        .icon(IconName::ChevronRight)
                                                        .ghost()
                                                        .on_click(move |_event, _window, cx| {
                                                            let p = p_drill.clone();
                                                            ws_drill.update(cx, |ws, cx| {
                                                                ws.drill_down(p, cx);
                                                            });
                                                        })
                                                        .into_any_element()
                                                } else {
                                                    Button::new(format!("tbl-open-{}", path.display()))
                                                        .icon(IconName::ExternalLink)
                                                        .label("Open")
                                                        .ghost()
                                                        .on_click(move |_event, _window, _cx| {
                                                            Workspace::open_in_system_editor(&p_open_btn);
                                                        })
                                                        .into_any_element()
                                                },
                                            ),
                                        )
                                })),
                            ),
                    ),
            )
    }
}
