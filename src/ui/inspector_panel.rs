use std::fs;
use std::path::PathBuf;
use gpui::prelude::*;
use gpui::{
    Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    ParentElement, Render, Styled, Window, div,
};
use gpui_component::dock::{BasePanel, Panel, PanelEvent};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{
    ActiveTheme, StyledExt, h_flex, label::Label, v_flex,
};

use crate::model::fs_entry::FsEntry;
use crate::workspace::Workspace;

pub struct InspectorPanel {
    workspace: Entity<Workspace>,
    focus_handle: FocusHandle,
}

impl InspectorPanel {
    pub fn new(workspace: Entity<Workspace>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            workspace,
            focus_handle: cx.focus_handle(),
        }
    }

    fn read_preview_lines(path: &PathBuf, max_lines: usize) -> Option<Vec<String>> {
        if let Ok(content) = fs::read_to_string(path) {
            let lines: Vec<String> = content.lines().take(max_lines).map(|s| s.to_string()).collect();
            Some(lines)
        } else {
            None
        }
    }
}

impl EventEmitter<PanelEvent> for InspectorPanel {}

impl Focusable for InspectorPanel {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BasePanel for InspectorPanel {
    fn panel_name(&self) -> &'static str {
        "InspectorPanel"
    }
}

impl Panel for InspectorPanel {
    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        "Inspector & Preview"
    }
}

impl Render for InspectorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (selected_path, current_path) = {
            let ws = self.workspace.read(cx);
            (ws.selected_path.clone(), ws.current_path.clone())
        };

        // If no file explicitly selected, inspect current directory
        let inspect_target = selected_path.as_ref().unwrap_or(&current_path);
        let entry = FsEntry::from_path(inspect_target, true, 1, true);

        let ws = self.workspace.clone();
        let target_path = inspect_target.clone();

        v_flex()
            .id("inspector-panel-root")
            .size_full()
            .p_3()
            .gap_3()
            .overflow_y_scroll()
            .bg(cx.theme().background)
            // Header card
            .child(
                v_flex()
                    .w_full()
                    .p_3()
                    .gap_2()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().secondary.opacity(0.3))
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .px_2()
                                            .py_0p5()
                                            .rounded_sm()
                                            .bg(cx.theme().primary.opacity(0.15))
                                            .text_xs()
                                            .font_bold()
                                            .text_color(cx.theme().primary)
                                            .child(entry.category.display_badge()),
                                    )
                                    .child(
                                        Label::new(entry.name.clone())
                                            .font_bold()
                                            .text_sm(),
                                    ),
                            ),
                    )
                    .child(
                        Label::new(entry.path.to_string_lossy().to_string())
                            .text_xs()
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            // Metadata table
            .child(
                v_flex()
                    .w_full()
                    .p_3()
                    .gap_2()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(Label::new("Properties").font_bold().text_xs())
                    .child(
                        h_flex()
                            .justify_between()
                            .text_xs()
                            .child(Label::new("Type:").text_color(cx.theme().muted_foreground))
                            .child(Label::new(if entry.is_dir { "Directory" } else { "File" })),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .text_xs()
                            .child(Label::new("Size:").text_color(cx.theme().muted_foreground))
                            .child(Label::new(entry.format_size())),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .text_xs()
                            .child(Label::new("Modified:").text_color(cx.theme().muted_foreground))
                            .child(Label::new(entry.format_modified())),
                    )
                    .when(entry.is_dir, |el| {
                        el.child(
                            h_flex()
                                .justify_between()
                                .text_xs()
                                .child(Label::new("Items:").text_color(cx.theme().muted_foreground))
                                .child(Label::new(format!("{} children", entry.item_count))),
                        )
                    }),
            )
            // Action Buttons
            .child(
                v_flex()
                    .w_full()
                    .gap_2()
                    .child(
                        Button::new("btn-open-system")
                            .label("Open in Default App")
                            .on_click(cx.listener({
                                let p = target_path.clone();
                                move |_this, _event, _window, _cx| {
                                    Workspace::open_in_system_editor(&p);
                                }
                            })),
                    )
                    .child(
                        Button::new("btn-reveal-fm")
                            .label("Reveal in File Manager")
                            .ghost()
                            .on_click(cx.listener({
                                let p = target_path.clone();
                                move |_this, _event, _window, _cx| {
                                    Workspace::reveal_in_file_manager(&p);
                                }
                            })),
                    )
                    .when(selected_path.is_some(), |el| {
                        let ws_del = ws.clone();
                        let p_del = target_path.clone();
                        el.child(
                            Button::new("btn-delete-node")
                                .label("Delete Item")
                                .ghost()
                                .on_click(cx.listener(move |_this, _event, _window, cx| {
                                    let target = p_del.clone();
                                    ws_del.update(cx, |ws, cx| {
                                        let _ = ws.delete_entry(&target, cx);
                                    });
                                })),
                        )
                    }),
            )
            // Preview section
            .child(
                v_flex()
                    .w_full()
                    .flex_1()
                    .p_3()
                    .gap_2()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().secondary.opacity(0.15))
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(Label::new("Content Preview").font_bold().text_xs()),
                    )
                    .child({
                        if entry.is_dir {
                            v_flex()
                                .gap_1()
                                .children(entry.children.iter().take(20).map(|c| {
                                    h_flex()
                                        .justify_between()
                                        .text_xs()
                                        .child(Label::new(format!(
                                            "{} {}",
                                            if c.is_dir { "📁" } else { "📄" },
                                            c.name
                                        )))
                                        .child(Label::new(c.format_size()).text_color(cx.theme().muted_foreground))
                                }))
                                .into_any_element()
                        } else if let Some(lines) = Self::read_preview_lines(&target_path, 40) {
                            v_flex()
                                .gap_0p5()
                                .children(lines.into_iter().enumerate().map(|(line_no, line)| {
                                    h_flex()
                                        .gap_2()
                                        .text_xs()
                                        .child(
                                            Label::new(format!("{:3}", line_no + 1))
                                                .text_color(cx.theme().muted_foreground.opacity(0.5)),
                                        )
                                        .child(Label::new(line))
                                }))
                                .into_any_element()
                        } else {
                            Label::new("Binary or non-UTF-8 file.")
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .into_any_element()
                        }
                    }),
            )
    }
}
