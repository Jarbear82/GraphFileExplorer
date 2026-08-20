use std::collections::HashSet;
use std::path::PathBuf;
use gpui::prelude::*;
use gpui::{
    AnyElement, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    Render, Styled, Window, div, px,
};
use gpui_component::dock::{BasePanel, Panel, PanelEvent};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{
    ActiveTheme, Disableable, StyledExt, h_flex, label::Label, v_flex,
};

use crate::model::fs_entry::FsEntry;
use crate::workspace::Workspace;

pub struct FilesPanel {
    workspace: Entity<Workspace>,
    focus_handle: FocusHandle,
    expanded_paths: HashSet<PathBuf>,
}

impl FilesPanel {
    pub fn new(workspace: Entity<Workspace>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let root_path = workspace.read(cx).root_path.clone();
        let mut expanded = HashSet::new();
        expanded.insert(root_path);

        Self {
            workspace,
            focus_handle: cx.focus_handle(),
            expanded_paths: expanded,
        }
    }

    fn toggle_expand(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.expanded_paths.contains(&path) {
            self.expanded_paths.remove(&path);
        } else {
            self.expanded_paths.insert(path);
        }
        cx.notify();
    }

    fn render_tree_node(
        &self,
        entry: &FsEntry,
        depth: usize,
        selected_path: &Option<PathBuf>,
        current_path: &PathBuf,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let path = entry.path.clone();
        let is_dir = entry.is_dir;
        let is_expanded = self.expanded_paths.contains(&path);
        let is_selected = selected_path.as_ref() == Some(&path);
        let is_current_root = current_path == &path;
        let indent = depth as f32 * 14.0;

        let ws_click = self.workspace.clone();
        let ws_drill = self.workspace.clone();
        let node_path = path.clone();
        let target_for_click = path.clone();
        let target_for_dbl = path.clone();

        let mut child_nodes = Vec::new();
        if is_dir && is_expanded {
            let mut dir_entry = entry.clone();
            if !dir_entry.is_loaded {
                dir_entry.load_children(1, false);
            }
            for child in &dir_entry.children {
                child_nodes.push(self.render_tree_node(child, depth + 1, selected_path, current_path, cx));
            }
        }

        v_flex()
            .w_full()
            .child(
                h_flex()
                    .id(format!("tree-item-{}", path.display()))
                    .w_full()
                    .h(px(24.0))
                    .pl(px(indent + 6.0))
                    .pr_2()
                    .items_center()
                    .justify_between()
                    .rounded_sm()
                    .gap_1()
                    .hover(|s| s.bg(cx.theme().secondary.opacity(0.4)))
                    .when(is_selected, |s| s.bg(cx.theme().primary.opacity(0.18)))
                    .when(is_current_root, |s| s.font_bold().border_l_2().border_color(cx.theme().primary))
                    .on_click(cx.listener(move |_this, _event, _window, cx| {
                        let sel = target_for_click.clone();
                        ws_click.update(cx, |ws, cx| {
                            ws.select_path(Some(sel), cx);
                        });
                    }))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .child(
                                if is_dir {
                                    div()
                                        .id(format!("toggle-btn-{}", node_path.display()))
                                        .w(px(14.0))
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(if is_expanded { "▼" } else { "▶" })
                                        .on_click(cx.listener({
                                            let p = node_path.clone();
                                            move |this, _event, _window, cx| {
                                                this.toggle_expand(p.clone(), cx);
                                            }
                                        }))
                                        .into_any_element()
                                } else {
                                    div().w(px(14.0)).into_any_element()
                                },
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .child(if is_dir { "📁" } else { "📄" }),
                            )
                            .child(
                                Label::new(entry.name.clone())
                                    .text_xs()
                                    .text_color(if is_selected {
                                        cx.theme().primary
                                    } else {
                                        cx.theme().foreground
                                    }),
                            ),
                    )
                    .child(
                        if is_dir {
                            Button::new(format!("btn-drill-{}", path.display()))
                                .label("➔")
                                .ghost()
                                .on_click(cx.listener(move |_this, _event, _window, cx| {
                                    let target = target_for_dbl.clone();
                                    ws_drill.update(cx, |ws, cx| {
                                        ws.drill_down(target, cx);
                                    });
                                }))
                        } else {
                            Button::new(format!("btn-ext-{}", path.display()))
                                .label(entry.category.display_badge())
                                .ghost()
                                .disabled(true)
                        },
                    ),
            )
            .when(is_dir && is_expanded, |el| {
                el.children(child_nodes)
            })
            .into_any_element()
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
        let (root_path, selected_path, current_path, show_hidden) = {
            let ws = self.workspace.read(cx);
            (
                ws.root_path.clone(),
                ws.selected_path.clone(),
                ws.current_path.clone(),
                ws.show_hidden,
            )
        };

        let root_entry = FsEntry::from_path(&root_path, true, 1, show_hidden);
        let root_node = self.render_tree_node(&root_entry, 0, &selected_path, &current_path, cx);

        v_flex()
            .size_full()
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
                                        let ws = self.workspace.clone();
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
                                        let ws = self.workspace.clone();
                                        move |_this, _event, _window, cx| {
                                            ws.update(cx, |ws, cx| {
                                                let _ = ws.create_entry("new_folder", true, cx);
                                            });
                                        }
                                    })),
                            ),
                    ),
            )
            // Tree content
            .child(
                v_flex()
                    .id("files-tree-list")
                    .flex_1()
                    .w_full()
                    .p_1()
                    .overflow_y_scroll()
                    .child(root_node),
            )
    }
}
