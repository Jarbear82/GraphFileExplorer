use std::path::{Path, PathBuf};
use gpui::prelude::*;
use gpui::{Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render, Styled, Window, div, px};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputState};
use gpui_component::{ActiveTheme, Disableable, Icon, IconName, Sizable, StyledExt, h_flex};

use crate::workspace::Workspace;

pub struct Breadcrumbs {
    workspace: Entity<Workspace>,
    focus_handle: FocusHandle,
    search_input: Entity<InputState>,
}

impl Breadcrumbs {
    pub fn new(
        workspace: Entity<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Filter items..."));

        Self {
            workspace,
            focus_handle: cx.focus_handle(),
            search_input,
        }
    }
}

impl Focusable for Breadcrumbs {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Breadcrumbs {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (current_path, root_path, can_go_up, can_go_back, can_go_forward, show_hidden) = {
            let ws = self.workspace.read(cx);
            (
                ws.current_path.clone(),
                ws.root_path.clone(),
                ws.current_path.parent().is_some() && ws.current_path != ws.root_path,
                ws.can_go_back(),
                ws.can_go_forward(),
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
            .h(px(38.0))
            .px_3()
            .py_1()
            .gap_1p5()
            .items_center()
            .justify_between()
            .bg(cx.theme().secondary.opacity(0.35))
            .border_b_1()
            .border_color(cx.theme().border)
            // Left: History Navigation + Clickable Ancestor Crumbs
            .child(
                h_flex()
                    .items_center()
                    .gap_1()
                    // History Back
                    .child(
                        Button::new("btn-nav-back")
                            .icon(IconName::ArrowLeft)
                            .ghost()
                            .disabled(!can_go_back)
                            .on_click(cx.listener({
                                let ws = ws.clone();
                                move |_this, _event, _window, cx| {
                                    ws.update(cx, |ws, cx| {
                                        ws.navigate_back(cx);
                                    });
                                }
                            })),
                    )
                    // History Forward
                    .child(
                        Button::new("btn-nav-forward")
                            .icon(IconName::ArrowRight)
                            .ghost()
                            .disabled(!can_go_forward)
                            .on_click(cx.listener({
                                let ws = ws.clone();
                                move |_this, _event, _window, cx| {
                                    ws.update(cx, |ws, cx| {
                                        ws.navigate_forward(cx);
                                    });
                                }
                            })),
                    )
                    // Up to parent
                    .child(
                        Button::new("btn-nav-up")
                            .icon(IconName::ArrowUp)
                            .label("Up")
                            .ghost()
                            .disabled(!can_go_up)
                            .on_click(cx.listener({
                                let ws = ws.clone();
                                move |_this, _event, _window, cx| {
                                    ws.update(cx, |ws, cx| {
                                        ws.navigate_up(cx);
                                    });
                                }
                            })),
                    )
                    .child(div().w(px(1.0)).h(px(18.0)).bg(cx.theme().border).mx_1())
                    // Clickable Breadcrumbs Segments
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .children(segments.into_iter().enumerate().map(|(idx, (name, path))| {
                                let is_last = path == current_path;
                                let ws_crumb = ws.clone();
                                let target_path = path.clone();

                                h_flex()
                                    .items_center()
                                    .gap_1()
                                    .when(idx > 0, |el| {
                                        el.child(Icon::new(IconName::ChevronRight).xsmall().text_color(cx.theme().muted_foreground))
                                    })
                                    .child(
                                        Button::new(format!("crumb-{}", idx))
                                            .icon(if path.is_dir() { IconName::Folder } else { IconName::File })
                                            .label(name)
                                            .ghost()
                                            .when(is_last, |b| b.font_bold().border_b_2().border_color(cx.theme().primary))
                                            .on_click(cx.listener(move |_this, _event, _window, cx| {
                                                if !is_last {
                                                    let p = target_path.clone();
                                                    ws_crumb.update(cx, |ws, cx| {
                                                        ws.drill_down(p, cx);
                                                    });
                                                }
                                            })),
                                    )
                            })),
                    ),
            )
            // Right: Live Filter Input + Toggles & Refresh
            .child(
                h_flex()
                    .items_center()
                    .gap_1p5()
                    .child(
                        div()
                            .w(px(180.0))
                            .child(Input::new(&self.search_input).cleanable(true)),
                    )
                    .child(
                        Button::new("btn-toggle-filter")
                            .icon(IconName::Search)
                            .label("Filter")
                            .ghost()
                            .on_click(cx.listener({
                                let ws = ws.clone();
                                let input = self.search_input.clone();
                                move |_this, _event, _window, cx| {
                                    let query = input.read(cx).text().to_string();
                                    ws.update(cx, |ws, cx| {
                                        ws.set_filter_query(query, cx);
                                    });
                                }
                            })),
                    )
                    .child(
                        Button::new("btn-toggle-hidden")
                            .icon(if show_hidden { IconName::Eye } else { IconName::EyeOff })
                            .label("Hidden")
                            .ghost()
                            .on_click(cx.listener({
                                let ws = ws.clone();
                                move |_this, _event, _window, cx| {
                                    ws.update(cx, |ws, cx| {
                                        ws.toggle_hidden(cx);
                                    });
                                }
                            })),
                    )
                    .child(
                        Button::new("btn-refresh")
                            .icon(IconName::Redo)
                            .ghost()
                            .on_click(cx.listener({
                                let ws = ws.clone();
                                move |_this, _event, _window, cx| {
                                    ws.update(cx, |ws, cx| {
                                        ws.refresh(cx);
                                    });
                                }
                            })),
                    ),
            )
    }
}
