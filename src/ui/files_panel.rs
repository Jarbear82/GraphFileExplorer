use std::path::{Path, PathBuf};
use gpui::prelude::*;
use gpui::{
    Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    Render, Styled, Window, div, px,
};
use gpui_component::dock::{BasePanel, Panel, PanelEvent};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::list::ListItem;
use gpui_component::tree::{TreeEntry, TreeItem, TreeState, tree};
use gpui_component::{
    ActiveTheme, IconName, StyledExt, h_flex, label::Label, v_flex,
};

use crate::workspace::Workspace;

fn build_tree_items(root_path: &Path, show_hidden: bool, depth_limit: usize) -> Vec<TreeItem> {
    let mut items = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());

            if !show_hidden && name.starts_with('.') {
                continue;
            }

            let id = path.to_string_lossy().to_string();
            let is_dir = path.is_dir();

            let mut item = TreeItem::new(id, name);
            if is_dir && depth_limit > 0 {
                let children = build_tree_items(&path, show_hidden, depth_limit - 1);
                item = item.children(children);
            }
            items.push(item);
        }
    }

    items.sort_by(|a, b| {
        b.is_folder()
            .cmp(&a.is_folder())
            .then(a.label.cmp(&b.label))
    });

    items
}

pub struct FilesPanel {
    workspace: Entity<Workspace>,
    focus_handle: FocusHandle,
    tree_state: Entity<TreeState>,
    last_root: PathBuf,
}

impl FilesPanel {
    pub fn new(workspace: Entity<Workspace>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let root_path = workspace.read(cx).root_path.clone();
        let show_hidden = workspace.read(cx).show_hidden;

        let root_name = root_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| root_path.to_string_lossy().to_string());

        let root_children = build_tree_items(&root_path, show_hidden, 2);
        let root_item = TreeItem::new(root_path.to_string_lossy().to_string(), root_name)
            .expanded(true)
            .children(root_children);

        let tree_state = cx.new(|cx| TreeState::new(cx).items(vec![root_item]));

        Self {
            workspace,
            focus_handle: cx.focus_handle(),
            tree_state,
            last_root: root_path,
        }
    }

    pub fn reload_tree(&mut self, cx: &mut Context<Self>) {
        let root_path = self.workspace.read(cx).root_path.clone();
        let show_hidden = self.workspace.read(cx).show_hidden;

        let root_name = root_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| root_path.to_string_lossy().to_string());

        let root_children = build_tree_items(&root_path, show_hidden, 2);
        let root_item = TreeItem::new(root_path.to_string_lossy().to_string(), root_name)
            .expanded(true)
            .children(root_children);

        self.last_root = root_path;
        self.tree_state.update(cx, |state, cx| {
            state.set_items(vec![root_item], cx);
        });
        cx.notify();
    }
}

impl EventEmitter<PanelEvent> for FilesPanel {}

impl Focusable for FilesPanel {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BasePanel for FilesPanel {
    fn panel_name(&self) -> &'static str {
        "FilesPanel"
    }
}

impl Panel for FilesPanel {
    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        "Files Explorer"
    }
}

impl Render for FilesPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (root_path, _selected_path) = {
            let ws = self.workspace.read(cx);
            (ws.root_path.clone(), ws.selected_path.clone())
        };

        if self.last_root != root_path {
            self.reload_tree(cx);
        }

        let ws = self.workspace.clone();

        v_flex()
            .size_full()
            .overflow_hidden()
            .bg(cx.theme().background)
            // Toolbar header
            .child(
                h_flex()
                    .w_full()
                    .h(px(32.0))
                    .px_2()
                    .py_1()
                    .gap_1()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().secondary.opacity(0.3))
                    .child(
                        Label::new(
                            root_path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "Workspace".to_string()),
                        )
                        .font_bold()
                        .text_xs(),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("btn-new-file")
                                    .label("+ File")
                                    .ghost()
                                    .on_click(cx.listener({
                                        let ws = ws.clone();
                                        move |_this, _event, _window, cx| {
                                            ws.update(cx, |ws, cx| {
                                                let _ = ws.create_entry("new_file.txt", false, cx);
                                            });
                                        }
                                    })),
                            )
                            .child(
                                Button::new("btn-new-folder")
                                    .label("+ Folder")
                                    .ghost()
                                    .on_click(cx.listener({
                                        let ws = ws.clone();
                                        move |_this, _event, _window, cx| {
                                            ws.update(cx, |ws, cx| {
                                                let _ = ws.create_entry("new_folder", true, cx);
                                            });
                                        }
                                    })),
                            ),
                    ),
            )
            // Official gpui_component Tree Component (keyboard nav, expand/collapse, virtualized list)
            .child(
                div()
                    .id("tree-view-wrapper")
                    .flex_1()
                    .min_h(px(0.0))
                    .size_full()
                    .overflow_hidden()
                    .child(
                        tree(&self.tree_state, {
                            let ws = ws.clone();
                            move |ix, entry: &TreeEntry, is_selected, _window, cx| {
                                let path_str = entry.item().id.to_string();
                                let path = PathBuf::from(&path_str);
                                let name = entry.item().label.to_string();
                                let is_dir = entry.is_folder();
                                let is_expanded = entry.is_expanded();

                                let icon = if !is_dir {
                                    IconName::File
                                } else if is_expanded {
                                    IconName::FolderOpen
                                } else {
                                    IconName::Folder
                                };

                                let ws_click = ws.clone();
                                let ws_drill = ws.clone();
                                let p_click = path.clone();
                                let p_drill = path.clone();

                                ListItem::new(ix)
                                    .w_full()
                                    .h(px(26.0))
                                    .rounded(cx.theme().radius)
                                    .px_2()
                                    .pl(px(14.0) * entry.depth() + px(6.0))
                                    .selected(is_selected)
                                    .on_click(move |_event, _window, cx| {
                                        let p = p_click.clone();
                                        ws_click.update(cx, |ws, cx| {
                                            ws.select_path(Some(p), cx);
                                        });
                                    })
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .items_center()
                                            .justify_between()
                                            .gap_1p5()
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap_1p5()
                                                    .child(icon)
                                                    .child(
                                                        Label::new(name)
                                                            .text_xs()
                                                            .text_color(if is_selected {
                                                                cx.theme().primary
                                                            } else {
                                                                cx.theme().foreground
                                                            }),
                                                    ),
                                            )
                                            .when(is_dir, |el| {
                                                el.child(
                                                    Button::new(format!("btn-drill-{}", path_str))
                                                        .label("➔")
                                                        .ghost()
                                                        .on_click(move |_event, _window, cx| {
                                                            let p = p_drill.clone();
                                                            ws_drill.update(cx, |ws, cx| {
                                                                ws.drill_down(p, cx);
                                                            });
                                                        }),
                                                )
                                            }),
                                    )
                            }
                        })
                        .size_full(),
                    ),
            )
    }
}
