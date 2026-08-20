use gpui::prelude::*;
use gpui::{
    Context, Entity, FocusHandle, Focusable, IntoElement, KeyDownEvent,
    ParentElement, Render, Styled, Window, div, px,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{
    ActiveTheme, Disableable, StyledExt, h_flex, label::Label, v_flex,
};

use crate::model::layout::{LayoutKind, LayoutNode};
use crate::workspace::Workspace;

pub struct GraphView {
    workspace: Entity<Workspace>,
    focus_handle: FocusHandle,
}

impl GraphView {
    pub fn new(workspace: Entity<Workspace>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            workspace,
            focus_handle: cx.focus_handle(),
        }
    }

    fn render_nested_preview(&self, node: &LayoutNode, cx: &mut Context<Self>) -> impl IntoElement {
        let max_show = 6;
        let preview_items = node.children.iter().take(max_show);

        h_flex()
            .w_full()
            .flex_wrap()
            .gap_1()
            .p_1()
            .rounded_sm()
            .bg(cx.theme().background.opacity(0.6))
            .border_1()
            .border_color(cx.theme().border.opacity(0.5))
            .children(preview_items.map(|child| {
                div()
                    .px_1()
                    .py_0p5()
                    .rounded_sm()
                    .bg(if child.is_dir {
                        cx.theme().secondary.opacity(0.8)
                    } else {
                        cx.theme().primary.opacity(0.12)
                    })
                    .text_xs()
                    .text_color(if child.is_dir {
                        cx.theme().foreground
                    } else {
                        cx.theme().primary
                    })
                    .child(if child.is_dir {
                        format!("📁 {}", child.name)
                    } else {
                        format!("📄 {}", child.name)
                    })
            }))
            .when(node.children.len() > max_show, |el| {
                el.child(
                    div()
                        .px_1()
                        .py_0p5()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("+{}", node.children.len() - max_show)),
                )
            })
    }

    fn render_compound_node(
        &self,
        node: &LayoutNode,
        is_selected: bool,
        is_radial: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let ws = self.workspace.clone();
        let target_path = node.path.clone();
        let drill_path = node.path.clone();
        let is_dir = node.is_dir;

        v_flex()
            .id(format!("card-{}", node.id))
            .when(is_radial, |s| s.absolute().left(px(node.x)).top(px(node.y)))
            .w(px(node.width))
            .min_h(px(node.height))
            .p_2()
            .gap_1()
            .rounded_md()
            .border_1()
            .border_color(if is_selected {
                cx.theme().primary
            } else {
                cx.theme().border
            })
            .bg(if is_selected {
                cx.theme().primary.opacity(0.08)
            } else if is_dir {
                cx.theme().secondary.opacity(0.35)
            } else {
                cx.theme().background
            })
            .shadow_sm()
            .hover(|s| s.border_color(cx.theme().primary.opacity(0.7)).shadow_md())
            .cursor_pointer()
            .on_click(cx.listener({
                let p = target_path.clone();
                move |_this, _event, _window, cx| {
                    ws.update(cx, |ws, cx| {
                        ws.select_path(Some(p.clone()), cx);
                    });
                }
            }))
            // Node Header
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1p5()
                            .child(
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
                                    .child(node.category.display_badge()),
                            )
                            .child(
                                Label::new(node.name.clone())
                                    .font_semibold()
                                    .text_xs(),
                            ),
                    )
                    .child(
                        if is_dir {
                            Button::new(format!("btn-enter-{}", node.id))
                                .label("➔")
                                .ghost()
                                .on_click(cx.listener({
                                    let p = drill_path.clone();
                                    let ws = self.workspace.clone();
                                    move |_this, _event, _window, cx| {
                                        ws.update(cx, |ws, cx| {
                                            ws.drill_down(p.clone(), cx);
                                        });
                                    }
                                }))
                                .into_any_element()
                        } else {
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(crate::model::fs_entry::format_bytes(node.size_bytes))
                                .into_any_element()
                        },
                    ),
            )
            // Nested Child Preview (Algorithm 2 Top-down mini-box rendering)
            .when(is_dir && !node.children.is_empty(), |el| {
                el.child(self.render_nested_preview(node, cx))
            })
            .when(is_dir && node.children.is_empty(), |el| {
                el.child(
                    div()
                        .w_full()
                        .py_1()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{} items", node.item_count)),
                )
            })
    }

    fn handle_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let ws = self.workspace.clone();
        let key = event.keystroke.key.as_str();

        match key {
            "enter" => {
                let selected = ws.read(cx).selected_path.clone();
                if let Some(path) = selected {
                    if path.is_dir() {
                        ws.update(cx, |ws, cx| {
                            ws.drill_down(path, cx);
                        });
                    } else {
                        Workspace::open_in_system_editor(&path);
                    }
                }
            }
            "backspace" => {
                ws.update(cx, |ws, cx| {
                    ws.navigate_up(cx);
                });
            }
            "delete" => {
                let selected = ws.read(cx).selected_path.clone();
                if let Some(path) = selected {
                    ws.update(cx, |ws, cx| {
                        let _ = ws.delete_entry(&path, cx);
                    });
                }
            }
            _ => {}
        }
    }
}

impl Focusable for GraphView {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GraphView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (selected_path, layout_result, layout_kind, current_path) = {
            let ws = self.workspace.read(cx);
            (
                ws.selected_path.clone(),
                ws.layout_result.clone(),
                ws.layout_kind,
                ws.current_path.clone(),
            )
        };

        let ws = self.workspace.clone();
        let is_radial = layout_kind == LayoutKind::RadialBalloonTree;

        // Render nodes
        let mut node_elements = Vec::new();
        if let Some(layout) = &layout_result {
            for child in &layout.root_node.children {
                let is_sel = selected_path.as_ref() == Some(&child.path);
                node_elements.push(self.render_compound_node(child, is_sel, is_radial, cx).into_any_element());
            }
        }

        v_flex()
            .id("graph-view-canvas-root")
            .size_full()
            .bg(cx.theme().background)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_key_down(event, window, cx);
            }))
            // Canvas Toolbar
            .child(
                h_flex()
                    .w_full()
                    .h(px(38.0))
                    .px_3()
                    .py_1()
                    .gap_2()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().secondary.opacity(0.2))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Label::new(format!(
                                    "📂 {}",
                                    current_path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_else(|| "Root".to_string())
                                ))
                                .font_bold()
                                .text_sm(),
                            ),
                    )
                    // Algorithm Selector buttons
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .children(LayoutKind::all().iter().map(|&kind| {
                                let is_active = kind == layout_kind;
                                let is_avail = kind.is_available();
                                let ws_kind = ws.clone();

                                Button::new(format!("layout-btn-{}", kind.name()))
                                    .label(kind.name())
                                    .ghost()
                                    .when(is_active, |b| b.font_bold().border_1().border_color(cx.theme().primary))
                                    .disabled(!is_avail)
                                    .on_click(cx.listener(move |_this, _event, _window, cx| {
                                        ws_kind.update(cx, |ws, cx| {
                                            ws.set_layout_kind(kind, cx);
                                        });
                                    }))
                            })),
                    ),
            )
            // Compound Graph Canvas Grid / Radial Orbit Area
            .child(
                div()
                    .id("graph-canvas-scroll")
                    .flex_1()
                    .size_full()
                    .p_4()
                    .overflow_y_scroll()
                    .child({
                        if let Some(layout) = &layout_result {
                            if layout.root_node.children.is_empty() {
                                v_flex()
                                    .size_full()
                                    .justify_center()
                                    .items_center()
                                    .gap_2()
                                    .child(Label::new("Empty Directory").font_bold())
                                    .child(
                                        Label::new("This folder contains no matching files or subdirectories.")
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                    .into_any_element()
                            } else if is_radial {
                                // Radial Balloon Tree Orbit Canvas
                                div()
                                    .id("radial-balloon-canvas")
                                    .relative()
                                    .w(px(layout.total_width))
                                    .h(px(layout.total_height))
                                    // Central Core Hub Node (Fig 8 in Paper)
                                    .child(
                                        v_flex()
                                            .absolute()
                                            .left(px(layout.root_node.x))
                                            .top(px(layout.root_node.y))
                                            .w(px(layout.root_node.width))
                                            .h(px(layout.root_node.height))
                                            .p_2()
                                            .rounded_full()
                                            .border_2()
                                            .border_color(cx.theme().primary)
                                            .bg(cx.theme().primary.opacity(0.12))
                                            .justify_center()
                                            .items_center()
                                            .shadow_md()
                                            .child(
                                                Label::new("🔴 Central Hub")
                                                    .font_bold()
                                                    .text_xs()
                                                    .text_color(cx.theme().primary),
                                            )
                                            .child(
                                                Label::new(format!("{} children", layout.root_node.children.len()))
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground),
                                            ),
                                    )
                                    .children(node_elements)
                                    .into_any_element()
                            } else {
                                // Grid Flow Canvas (NativeTopDown)
                                h_flex()
                                    .w_full()
                                    .flex_wrap()
                                    .gap_3()
                                    .children(node_elements)
                                    .into_any_element()
                            }
                        } else {
                            v_flex()
                                .size_full()
                                .justify_center()
                                .items_center()
                                .child(Label::new("Loading layout..."))
                                .into_any_element()
                        }
                    }),
            )
    }
}
