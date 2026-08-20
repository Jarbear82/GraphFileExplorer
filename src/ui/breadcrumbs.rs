use gpui::prelude::*;
use gpui::{Context, Entity, IntoElement, ParentElement, Render, Styled, Window, px};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{ActiveTheme, Disableable, StyledExt, h_flex, label::Label};
use std::path::{Path, PathBuf};

use crate::workspace::Workspace;

pub struct Breadcrumbs {
    workspace: Entity<Workspace>,
}

impl Breadcrumbs {
    pub fn new(
        workspace: Entity<Workspace>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self { workspace }
    }
}

impl Render for Breadcrumbs {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (current_path, root_path, can_go_up, show_hidden) = {
            let ws = self.workspace.read(cx);
            (
                ws.current_path.clone(),
                ws.root_path.clone(),
                ws.current_path.parent().is_some() && ws.current_path != ws.root_path,
                ws.show_hidden,
            )
        };

        // Compute path segments from root to current
        let mut segments: Vec<(String, PathBuf)> = Vec::new();
        let mut curr: &Path = &current_path;

        while let Some(parent) = curr.parent() {
            let name = curr
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| curr.to_string_lossy().to_string());
            segments.push((name, curr.to_path_buf()));

            if curr == root_path {
                break;
            }
            curr = parent;
        }

        segments.reverse();
        if segments.is_empty() {
            segments.push((
                current_path.to_string_lossy().to_string(),
                current_path.clone(),
            ));
        }

        let ws = self.workspace.clone();

        h_flex()
            .w_full()
            .h(px(36.0))
            .px_3()
            .py_1()
            .gap_1()
            .items_center()
            .bg(cx.theme().secondary.opacity(0.35))
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                Button::new("btn-nav-up")
                    .label("▲ Up")
                    .disabled(!can_go_up)
                    .on_click(cx.listener(move |_this, _event, _window, cx| {
                        ws.update(cx, |ws, cx| {
                            ws.navigate_up(cx);
                        });
                    })),
            )
            .child(
                h_flex()
                    .flex_1()
                    .items_center()
                    .gap_1()
                    .overflow_hidden()
                    .children(segments.into_iter().enumerate().map(|(idx, (name, path))| {
                        let ws = self.workspace.clone();
                        let target_path = path.clone();
                        let is_last = target_path == current_path;

                        h_flex()
                            .items_center()
                            .gap_1()
                            .when(idx > 0, |el| {
                                el.child(Label::new("/").text_color(cx.theme().muted_foreground))
                            })
                            .child(
                                Button::new(format!("crumb-{}", path.display()))
                                    .label(name)
                                    .ghost()
                                    .when(is_last, |b| b.font_bold())
                                    .on_click(cx.listener(move |_this, _event, _window, cx| {
                                        let target = target_path.clone();
                                        ws.update(cx, |ws, cx| {
                                            ws.drill_down(target, cx);
                                        });
                                    })),
                            )
                    })),
            )
            .child(
                Button::new("btn-toggle-hidden")
                    .label(if show_hidden {
                        "Hidden: ON"
                    } else {
                        "Hidden: OFF"
                    })
                    .ghost()
                    .on_click(cx.listener({
                        let ws = self.workspace.clone();
                        move |_this, _event, _window, cx| {
                            ws.update(cx, |ws, cx| {
                                ws.toggle_hidden(cx);
                            });
                        }
                    })),
            )
            .child(
                Button::new("btn-refresh")
                    .label("↻")
                    .ghost()
                    .on_click(cx.listener({
                        let ws = self.workspace.clone();
                        move |_this, _event, _window, cx| {
                            ws.update(cx, |ws, cx| {
                                ws.refresh(cx);
                            });
                        }
                    })),
            )
    }
}
