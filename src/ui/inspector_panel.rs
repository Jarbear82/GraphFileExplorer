use std::fs;
use std::path::PathBuf;
use gpui::prelude::*;
use gpui::{
    Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    ParentElement, Render, Styled, Window, div, px,
};
use gpui_component::dock::{BasePanel, Panel, PanelEvent};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{
    ActiveTheme, IconName, StyledExt, h_flex, label::Label, v_flex,
};

use crate::model::fs_entry::{format_bytes, FsEntry};
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
            // Header Card
            .child(
                v_flex()
                    .w_full()
                    .p_3()
                    .gap_2()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().secondary.opacity(0.35))
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(if entry.is_dir {
                                        IconName::Folder
                                    } else {
                                        IconName::File
                                    })
                                    .child(
                                        Label::new(entry.name.clone())
                                            .font_bold()
                                            .text_sm(),
                                    ),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_sm()
                                    .text_xs()
                                    .font_bold()
                                    .bg(if entry.is_dir {
                                        cx.theme().secondary
                                    } else {
                                        cx.theme().primary.opacity(0.18)
                                    })
                                    .text_color(if entry.is_dir {
                                        cx.theme().foreground
                                    } else {
                                        cx.theme().primary
                                    })
                                    .child(entry.category.display_badge()),
                            ),
                    )
                    .child(
                        Label::new(entry.path.to_string_lossy().to_string())
                            .text_xs()
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            // Quick Action Buttons Bar
            .child(
                h_flex()
                    .w_full()
                    .gap_1p5()
                    .flex_wrap()
                    .child(
                        Button::new("btn-inspect-open")
                            .label(if entry.is_dir { "➔ Drill In" } else { "⚡ Open Editor" })
                            .primary()
                            .on_click(cx.listener({
                                let p = target_path.clone();
                                let is_dir = entry.is_dir;
                                let ws_open = ws.clone();
                                move |_this, _event, _window, cx| {
                                    if is_dir {
                                        ws_open.update(cx, |ws, cx| {
                                            ws.drill_down(p.clone(), cx);
                                        });
                                    } else {
                                        Workspace::open_in_system_editor(&p);
                                    }
                                }
                            })),
                    )
                    .child(
                        Button::new("btn-inspect-reveal")
                            .label("📁 Reveal")
                            .ghost()
                            .on_click(cx.listener({
                                let p = target_path.clone();
                                move |_this, _event, _window, _cx| {
                                    Workspace::reveal_in_file_manager(&p);
                                }
                            })),
                    )
                    .child(
                        Button::new("btn-inspect-copy")
                            .label("📋 Copy Path")
                            .ghost()
                            .on_click(cx.listener({
                                let p = target_path.clone();
                                move |_this, _event, _window, cx| {
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(p.to_string_lossy().to_string()));
                                }
                            })),
                    )
                    .child(
                        Button::new("btn-inspect-delete")
                            .label("🗑 Delete")
                            .ghost()
                            .on_click(cx.listener({
                                let p = target_path.clone();
                                let ws_del = ws.clone();
                                move |_this, _event, _window, cx| {
                                    ws_del.update(cx, |ws, cx| {
                                        let _ = ws.delete_entry(&p, cx);
                                    });
                                }
                            })),
                    ),
            )
            // Metadata Properties Card
            .child(
                v_flex()
                    .w_full()
                    .p_3()
                    .gap_2()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().secondary.opacity(0.15))
                    .child(Label::new("Properties").font_bold().text_xs())
                    .child(
                        h_flex()
                            .justify_between()
                            .child(Label::new("Size").text_xs().text_color(cx.theme().muted_foreground))
                            .child(Label::new(if entry.is_dir { format!("{} items", entry.item_count) } else { format_bytes(entry.size_bytes) }).text_xs().font_semibold()),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .child(Label::new("Type").text_xs().text_color(cx.theme().muted_foreground))
                            .child(Label::new(format!("{:?}", entry.category)).text_xs()),
                    )
                    .when_some(entry.extension, |el, ext| {
                        el.child(
                            h_flex()
                                .justify_between()
                                .child(Label::new("Extension").text_xs().text_color(cx.theme().muted_foreground))
                                .child(Label::new(format!(".{ext}")).text_xs()),
                        )
                    })
                    .child(
                        h_flex()
                            .justify_between()
                            .child(Label::new("Symlink").text_xs().text_color(cx.theme().muted_foreground))
                            .child(Label::new(if entry.is_symlink { "Yes" } else { "No" }).text_xs()),
                    ),
            )
            // Code & Text Syntax-Highlighted Preview (or Directory Contents summary)
            .child(
                if !entry.is_dir {
                    let preview = Self::read_preview_lines(&entry.path, 150);
                    v_flex()
                        .w_full()
                        .gap_1()
                        .child(
                            h_flex()
                                .items_center()
                                .justify_between()
                                .child(Label::new("File Content Preview").font_bold().text_xs())
                                .child(
                                    Label::new(format!("Previewing {} lines", preview.as_ref().map(|p| p.len()).unwrap_or(0)))
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground),
                                ),
                        )
                        .child(
                            div()
                                .id("file-preview-scroll-box")
                                .w_full()
                                .max_h(px(320.0))
                                .p_2()
                                .rounded_md()
                                .border_1()
                                .border_color(cx.theme().border)
                                .bg(cx.theme().background)
                                .overflow_y_scroll()
                                .child(
                                    if let Some(lines) = preview {
                                        if lines.is_empty() {
                                            Label::new("(Empty file)").text_xs().text_color(cx.theme().muted_foreground).into_any_element()
                                        } else {
                                            v_flex()
                                                .w_full()
                                                .gap_0p5()
                                                .children(lines.into_iter().enumerate().map(|(num, line)| {
                                                    h_flex()
                                                        .w_full()
                                                        .gap_2()
                                                        .child(
                                                            div()
                                                                .w(px(32.0))
                                                                .text_right()
                                                                .text_xs()
                                                                .text_color(cx.theme().muted_foreground.opacity(0.6))
                                                                .child(format!("{}", num + 1)),
                                                        )
                                                        .child(
                                                            Label::new(line)
                                                                .text_xs(),
                                                        )
                                                }))
                                                .into_any_element()
                                        }
                                    } else {
                                        Label::new("(Binary or non-UTF8 file)").text_xs().text_color(cx.theme().muted_foreground).into_any_element()
                                    }
                                ),
                        )
                        .into_any_element()
                } else {
                    v_flex()
                        .w_full()
                        .gap_1()
                        .child(Label::new(format!("Folder Items ({})", entry.children.len())).font_bold().text_xs())
                        .child(
                            v_flex()
                                .id("folder-children-scroll-box")
                                .w_full()
                                .max_h(px(240.0))
                                .p_2()
                                .gap_1()
                                .rounded_md()
                                .border_1()
                                .border_color(cx.theme().border)
                                .bg(cx.theme().secondary.opacity(0.15))
                                .overflow_y_scroll()
                                .children(entry.children.iter().map(|child| {
                                    h_flex()
                                        .w_full()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            h_flex()
                                                .items_center()
                                                .gap_1p5()
                                                .child(if child.is_dir { IconName::Folder } else { IconName::File })
                                                .child(Label::new(child.name.clone()).text_xs()),
                                        )
                                        .child(
                                            Label::new(if child.is_dir { "DIR".to_string() } else { format_bytes(child.size_bytes) })
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground),
                                        )
                                })),
                        )
                        .into_any_element()
                }
            )
    }
}
