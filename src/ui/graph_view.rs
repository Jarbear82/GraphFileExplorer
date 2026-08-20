use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, PathBuilder, Render,
    ScrollDelta, ScrollWheelEvent, Styled, Subscription, Window, canvas, div, point, px,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::dock::{BasePanel, Panel, PanelEvent};
use gpui_component::menu::{ContextMenuExt, PopupMenu, PopupMenuItem};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, Sizable, StyledExt, h_flex, label::Label, v_flex,
};
use std::path::PathBuf;

use crate::model::layout::{LayoutKind, LayoutNode, LayoutResult};
use crate::workspace::Workspace;

pub struct GraphView {
    workspace: Entity<Workspace>,
    focus_handle: FocusHandle,
    zoom: f32,
    pan_x: f32,
    pan_y: f32,
    is_dragging: bool,
    drag_start: Option<(f32, f32)>,
    drag_initial_pan: (f32, f32),
    _workspace_subscription: Subscription,
}

impl GraphView {
    pub fn new(workspace: Entity<Workspace>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&workspace, |_this, _ws, cx| {
            cx.notify();
        });

        Self {
            workspace,
            focus_handle: cx.focus_handle(),
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            is_dragging: false,
            drag_start: None,
            drag_initial_pan: (0.0, 0.0),
            _workspace_subscription: subscription,
        }
    }

    pub fn zoom_in(&mut self, cx: &mut Context<Self>) {
        self.zoom = (self.zoom * 1.25).min(4.0);
        cx.notify();
    }

    pub fn zoom_out(&mut self, cx: &mut Context<Self>) {
        self.zoom = (self.zoom / 1.25).max(0.1);
        cx.notify();
    }

    pub fn reset_zoom(&mut self, cx: &mut Context<Self>) {
        self.zoom = 1.0;
        self.pan_x = 0.0;
        self.pan_y = 0.0;
        cx.notify();
    }

    pub fn pan_by(&mut self, dx: f32, dy: f32, cx: &mut Context<Self>) {
        self.pan_x += dx;
        self.pan_y += dy;
        cx.notify();
    }

    pub fn fit_to_view(&mut self, layout: &LayoutResult, cx: &mut Context<Self>) {
        let viewport_w = 900.0;
        let viewport_h = 600.0;

        let scale_x = if layout.total_width > 0.0 {
            viewport_w / layout.total_width
        } else {
            1.0
        };
        let scale_y = if layout.total_height > 0.0 {
            viewport_h / layout.total_height
        } else {
            1.0
        };

        let optimal_zoom = scale_x.min(scale_y).clamp(0.2, 2.0);
        self.zoom = optimal_zoom;
        self.pan_x = (viewport_w - (layout.total_width * optimal_zoom)) / 2.0;
        self.pan_y = (viewport_h - (layout.total_height * optimal_zoom)) / 2.0;
        cx.notify();
    }

    fn render_nested_preview(&self, node: &LayoutNode, cx: &App) -> impl IntoElement {
        let max_show = 4;
        let preview_items = node.children.iter().take(max_show);
        let ws = self.workspace.clone();

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
                let ws_sub = ws.clone();
                let child_path = child.path.clone();
                let child_is_dir = child.is_dir;

                div()
                    .id(format!("pill-{}", child_path.display()))
                    .px_1p5()
                    .py_0p5()
                    .rounded_sm()
                    .cursor_pointer()
                    .bg(if child_is_dir {
                        cx.theme().secondary.opacity(0.8)
                    } else {
                        cx.theme().primary.opacity(0.12)
                    })
                    .hover(|s| s.bg(cx.theme().primary.opacity(0.35)))
                    .on_click(move |_event, _window, cx| {
                        let p = child_path.clone();
                        ws_sub.update(cx, |ws, cx| {
                            if child_is_dir {
                                ws.drill_down(p, cx);
                            } else {
                                ws.select_path(Some(p), cx);
                            }
                        });
                    })
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .child(
                                Icon::new(if child_is_dir {
                                    IconName::Folder
                                } else {
                                    IconName::File
                                })
                                .xsmall()
                                .text_color(if child_is_dir {
                                    cx.theme().foreground
                                } else {
                                    cx.theme().primary
                                }),
                            )
                            .child(Label::new(child.name.clone()).text_xs().text_color(
                                if child_is_dir {
                                    cx.theme().foreground
                                } else {
                                    cx.theme().primary
                                },
                            )),
                    )
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
        is_expanded: bool,
        is_radial: bool,
        cx: &App,
    ) -> impl IntoElement {
        let ws = self.workspace.clone();
        let target_path = node.path.clone();
        let drill_path = node.path.clone();
        let is_dir = node.is_dir;

        // Size-based subtle indicator color
        let size_badge_color = if is_dir {
            cx.theme().foreground
        } else if node.size_bytes > 10 * 1024 * 1024 {
            cx.theme().warning
        } else if node.size_bytes > 1024 * 1024 {
            cx.theme().primary
        } else {
            cx.theme().muted_foreground
        };

        // Coordinates & sizes: dynamically scaled for radial canvas, natural base size for TopDown grid
        let (node_x, node_y, node_w, node_h) = if is_radial {
            (
                node.x * self.zoom,
                node.y * self.zoom,
                (node.width * self.zoom).max(60.0),
                (node.height * self.zoom).max(40.0),
            )
        } else {
            (node.x, node.y, node.width, node.height)
        };

        let ws_click = ws.clone();
        let p_card = target_path.clone();

        v_flex()
            .id(format!("card-{}", node.id))
            .when(is_radial, |s| s.absolute().left(px(node_x)).top(px(node_y)))
            .w(px(node_w))
            .min_h(px(node_h))
            .p_2()
            .gap_1()
            .rounded_md()
            .border_1()
            .border_color(if is_selected {
                cx.theme().primary
            } else if is_expanded {
                cx.theme().primary.opacity(0.8)
            } else {
                cx.theme().border
            })
            .bg(if is_selected {
                cx.theme().primary.opacity(0.12)
            } else if is_expanded {
                cx.theme().primary.opacity(0.08)
            } else if is_dir {
                cx.theme().secondary.opacity(0.4)
            } else {
                cx.theme().background
            })
            .shadow_sm()
            .hover(|s| s.border_color(cx.theme().primary.opacity(0.7)).shadow_md())
            .cursor_pointer()
            .on_click(move |_event, _window, cx| {
                let p = p_card.clone();
                ws_click.update(cx, |ws, cx| {
                    ws.select_path(Some(p), cx);
                });
            })
            .context_menu({
                let ws_ctx = ws.clone();
                let p_ctx = target_path.clone();
                let is_dir = is_dir;
                let is_expanded = is_expanded;
                move |menu: PopupMenu, _window, _cx| {
                    let p_open = p_ctx.clone();
                    let p_reveal = p_ctx.clone();
                    let p_copy = p_ctx.clone();
                    let p_del = p_ctx.clone();
                    let p_exp = p_ctx.clone();
                    let ws_open = ws_ctx.clone();
                    let ws_del = ws_ctx.clone();
                    let ws_exp = ws_ctx.clone();

                    let mut m = menu;
                    if is_dir {
                        m = m
                            .item(
                                PopupMenuItem::new(if is_expanded {
                                    "Collapse Subtree"
                                } else {
                                    "Expand Subtree"
                                })
                                .icon(if is_expanded {
                                    IconName::Minus
                                } else {
                                    IconName::Plus
                                })
                                .on_click(move |_event, _window, cx| {
                                    let p = p_exp.clone();
                                    ws_exp.update(cx, |ws, cx| {
                                        ws.toggle_expand(p, cx);
                                    });
                                }),
                            )
                            .item(
                                PopupMenuItem::new("Drill Down")
                                    .icon(IconName::ChevronRight)
                                    .on_click(move |_event, _window, cx| {
                                        let p = p_open.clone();
                                        ws_open.update(cx, |ws, cx| {
                                            ws.drill_down(p, cx);
                                        });
                                    }),
                            );
                    } else {
                        m = m.item(
                            PopupMenuItem::new("Open in Editor")
                                .icon(IconName::ExternalLink)
                                .on_click(move |_event, _window, _cx| {
                                    Workspace::open_in_system_editor(&p_open);
                                }),
                        );
                    }

                    m.item(
                        PopupMenuItem::new("Reveal in File Manager")
                            .icon(IconName::FolderOpen)
                            .on_click(move |_event, _window, _cx| {
                                Workspace::reveal_in_file_manager(&p_reveal);
                            }),
                    )
                    .item(
                        PopupMenuItem::new("Copy Full Path")
                            .icon(IconName::Copy)
                            .on_click(move |_event, _window, cx| {
                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                    p_copy.to_string_lossy().to_string(),
                                ));
                            }),
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
                            }),
                    )
                }
            })
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
                                Icon::new(if is_dir {
                                    if is_expanded {
                                        IconName::FolderOpen
                                    } else {
                                        IconName::Folder
                                    }
                                } else {
                                    IconName::File
                                })
                                .small(),
                            )
                            .child(Label::new(node.name.clone()).font_semibold().text_xs()),
                    )
                    .child(if is_dir {
                        let ws_drill = ws.clone();
                        let ws_exp = ws.clone();
                        let p_drill = drill_path.clone();
                        let p_exp = drill_path.clone();

                        h_flex()
                            .items_center()
                            .gap_0p5()
                            // Expand / Collapse in-place Radial Balloon button
                            .child(
                                Button::new(format!("btn-expand-{}", node.id))
                                    .icon(if is_expanded {
                                        IconName::Minus
                                    } else {
                                        IconName::Plus
                                    })
                                    .ghost()
                                    .on_click(move |_event, _window, cx| {
                                        let p = p_exp.clone();
                                        ws_exp.update(cx, |ws, cx| {
                                            ws.toggle_expand(p, cx);
                                        });
                                    }),
                            )
                            // Drill Down Navigation Button
                            .child(
                                Button::new(format!("btn-enter-{}", node.id))
                                    .icon(IconName::ChevronRight)
                                    .ghost()
                                    .on_click(move |_event, _window, cx| {
                                        let p = p_drill.clone();
                                        ws_drill.update(cx, |ws, cx| {
                                            ws.drill_down(p, cx);
                                        });
                                    }),
                            )
                            .into_any_element()
                    } else {
                        div()
                            .text_xs()
                            .text_color(size_badge_color)
                            .child(crate::model::fs_entry::format_bytes(node.size_bytes))
                            .into_any_element()
                    }),
            )
            // Nested Child Preview (when collapsed)
            .when(
                is_dir && !node.children.is_empty() && !is_expanded && (!is_radial || self.zoom > 0.45),
                |el| el.child(self.render_nested_preview(node, cx)),
            )
            .when(is_dir && is_expanded, |el| {
                el.child(
                    div()
                        .w_full()
                        .py_0p5()
                        .text_xs()
                        .text_color(cx.theme().primary)
                        .child(format!("Expanded ({} items)", node.children.len())),
                )
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

    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
                    }
                }
            }
            "e" | " " => {
                let selected = ws.read(cx).selected_path.clone();
                if let Some(path) = selected {
                    if path.is_dir() {
                        ws.update(cx, |ws, cx| {
                            ws.toggle_expand(path, cx);
                        });
                    }
                }
            }
            "backspace" => {
                ws.update(cx, |ws, cx| {
                    ws.navigate_up(cx);
                });
            }
            "=" | "+" => {
                self.zoom_in(cx);
            }
            "-" => {
                self.zoom_out(cx);
            }
            "0" => {
                self.reset_zoom(cx);
            }
            "left" => {
                self.pan_by(40.0, 0.0, cx);
            }
            "right" => {
                self.pan_by(-40.0, 0.0, cx);
            }
            "up" => {
                self.pan_by(0.0, 40.0, cx);
            }
            "down" => {
                self.pan_by(0.0, -40.0, cx);
            }
            _ => {}
        }
    }
}

impl EventEmitter<PanelEvent> for GraphView {}

impl Focusable for GraphView {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BasePanel for GraphView {
    fn panel_name(&self) -> &'static str {
        "GraphView"
    }
}

impl Panel for GraphView {
    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        "Graph Canvas"
    }
}

impl Render for GraphView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (current_path, selected_path, expanded_paths, layout_kind, layout_result) = {
            let ws = self.workspace.read(cx);
            (
                ws.current_path.clone(),
                ws.selected_path.clone(),
                ws.expanded_paths.clone(),
                ws.layout_kind,
                ws.layout_result.clone(),
            )
        };

        let ws = self.workspace.clone();
        let is_radial = layout_kind == LayoutKind::RadialBalloonTree;

        let node_elements: Vec<AnyElement> = if let Some(layout) = &layout_result {
            if is_radial {
                let mut elements = Vec::new();
                collect_radial_node_elements(
                    &layout.root_node,
                    &selected_path,
                    &expanded_paths,
                    &mut elements,
                    self,
                    cx,
                );
                elements
            } else {
                layout
                    .root_node
                    .children
                    .iter()
                    .map(|child| {
                        let is_sel = selected_path.as_ref() == Some(&child.path);
                        let is_exp = expanded_paths.contains(&child.path);
                        self.render_compound_node(child, is_sel, is_exp, is_radial, cx)
                            .into_any_element()
                    })
                    .collect()
            }
        } else {
            Vec::new()
        };

        let zoom = self.zoom;
        let pan_x = self.pan_x;
        let pan_y = self.pan_y;

        v_flex()
            .key_context("GraphView")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event, window, cx| {
                this.handle_key_down(event, window, cx);
            }))
            .size_full()
            .overflow_hidden()
            .bg(cx.theme().background)
            // Canvas Toolbar Header
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
                            .child(Icon::new(IconName::Folder).small())
                            .child(
                                Label::new(
                                    current_path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_else(|| "Root".to_string()),
                                )
                                .font_bold()
                                .text_sm(),
                            ),
                    )
                    // Algorithm Switcher
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
                                    .when(is_active, |b| {
                                        b.font_bold().border_1().border_color(cx.theme().primary)
                                    })
                                    .disabled(!is_avail)
                                    .on_click(cx.listener(move |_this, _event, _window, cx| {
                                        ws_kind.update(cx, |ws, cx| {
                                            ws.set_layout_kind(kind, cx);
                                        });
                                    }))
                            })),
                    ),
            )
            // Canvas Body: NativeTopDown fits viewport naturally with vertical scroll; RadialBalloon uses interactive canvas
            .child({
                if let Some(layout) = &layout_result {
                    if layout.root_node.children.is_empty() {
                        v_flex()
                            .size_full()
                            .justify_center()
                            .items_center()
                            .gap_2()
                            .child(
                                Icon::new(IconName::Inbox)
                                    .large()
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .child(Label::new("Empty Directory").font_bold())
                            .child(
                                Label::new(
                                    "This folder contains no matching files or subdirectories.",
                                )
                                .text_xs()
                                .text_color(cx.theme().muted_foreground),
                            )
                            .into_any_element()
                    } else if !is_radial {
                        // Native Top Down: Clean, natural responsive grid that fits the viewport with standard vertical scrolling
                        div()
                            .id("native-topdown-canvas-container")
                            .flex_1()
                            .size_full()
                            .overflow_y_scroll()
                            .p_4()
                            .child(
                                h_flex()
                                    .w_full()
                                    .flex_wrap()
                                    .gap_3()
                                    .children(node_elements),
                            )
                            .into_any_element()
                    } else {
                        // Radial Balloon Tree: Infinite-plane ZUI canvas with Mouse Dragging, Wheel Zooming & Floating Pill
                        let hub_x = layout.root_node.x * zoom;
                        let hub_y = layout.root_node.y * zoom;
                        let hub_w = (layout.root_node.width * zoom).max(80.0);
                        let hub_h = (layout.root_node.height * zoom).max(45.0);

                        div()
                            .id("radial-interactive-viewport")
                            .flex_1()
                            .min_w(px(0.0))
                            .min_h(px(0.0))
                            .size_full()
                            .overflow_hidden()
                            .relative()
                            .bg(cx.theme().background)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                                    this.is_dragging = true;
                                    let pos_x: f32 = event.position.x.into();
                                    let pos_y: f32 = event.position.y.into();
                                    this.drag_start = Some((pos_x, pos_y));
                                    this.drag_initial_pan = (this.pan_x, this.pan_y);
                                    cx.notify();
                                }),
                            )
                            .on_mouse_move(cx.listener(
                                |this, event: &MouseMoveEvent, _window, cx| {
                                    if this.is_dragging {
                                        if let Some((start_x, start_y)) = this.drag_start {
                                            let current_x: f32 = event.position.x.into();
                                            let current_y: f32 = event.position.y.into();
                                            let dx = current_x - start_x;
                                            let dy = current_y - start_y;
                                            this.pan_x = this.drag_initial_pan.0 + dx;
                                            this.pan_y = this.drag_initial_pan.1 + dy;
                                            cx.notify();
                                        }
                                    }
                                },
                            ))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _event: &MouseUpEvent, _window, cx| {
                                    this.is_dragging = false;
                                    this.drag_start = None;
                                    cx.notify();
                                }),
                            )
                            .on_scroll_wheel(cx.listener(
                                |this, event: &ScrollWheelEvent, _window, cx| {
                                    let delta_y = match event.delta {
                                        ScrollDelta::Pixels(p) => {
                                            let y: f32 = p.y.into();
                                            y
                                        }
                                        ScrollDelta::Lines(p) => p.y * 20.0,
                                    };

                                    if delta_y != 0.0 {
                                        if delta_y > 0.0 {
                                            this.zoom = (this.zoom * 1.12).min(3.5);
                                        } else {
                                            this.zoom = (this.zoom / 1.12).max(0.15);
                                        }
                                        cx.notify();
                                    }
                                },
                            ))
                            .child(
                                div()
                                    .id("radial-balloon-canvas-plane")
                                    .absolute()
                                    .left(px(pan_x))
                                    .top(px(pan_y))
                                    .w(px(layout.total_width * zoom))
                                    .h(px(layout.total_height * zoom))
                                    // Visual Connector Edges (Hub -> Children, and Parent -> Expanded Sub-Children)
                                    .child(
                                        canvas(
                                            move |_bounds, _window, _cx| (),
                                            {
                                                let root_children = layout.root_node.children.clone();
                                                let expanded = expanded_paths.clone();
                                                let zoom = zoom;
                                                move |bounds, _, window, cx| {
                                                    let hub_cx = hub_x + hub_w / 2.0;
                                                    let hub_cy = hub_y + hub_h / 2.0;
                                                    let hub_hw = hub_w / 2.0 + 1.0;
                                                    let hub_hh = hub_h / 2.0 + 1.0;

                                                    let mut segments = Vec::new();
                                                    collect_edge_segments(
                                                        hub_cx, hub_cy, hub_hw, hub_hh,
                                                        &root_children,
                                                        &expanded,
                                                        zoom,
                                                        &mut segments,
                                                    );

                                                    if !segments.is_empty() {
                                                        let mut builder = PathBuilder::stroke(px(1.5));
                                                        for ((sx, sy), (ex, ey)) in segments {
                                                            let p_start = point(bounds.origin.x + px(sx), bounds.origin.y + px(sy));
                                                            let p_end = point(bounds.origin.x + px(ex), bounds.origin.y + px(ey));
                                                            builder.move_to(p_start);
                                                            builder.line_to(p_end);
                                                        }

                                                        if let Ok(path) = builder.build() {
                                                            let line_color = cx.theme().primary.opacity(0.4);
                                                            window.paint_path(path, line_color);
                                                        }
                                                    }
                                                }
                                            },
                                        )
                                        .absolute()
                                        .size_full(),
                                    )
                                    // Central Core Hub Node
                                    .child(
                                        v_flex()
                                            .absolute()
                                            .left(px(hub_x))
                                            .top(px(hub_y))
                                            .w(px(hub_w))
                                            .h(px(hub_h))
                                            .p_2()
                                            .rounded_full()
                                            .border_2()
                                            .border_color(cx.theme().primary)
                                            .bg(cx.theme().primary.opacity(0.15))
                                            .justify_center()
                                            .items_center()
                                            .shadow_md()
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap_1()
                                                    .child(
                                                        Icon::new(IconName::FolderOpen)
                                                            .small()
                                                            .text_color(cx.theme().primary),
                                                    )
                                                    .child(
                                                        Label::new("Hub")
                                                            .font_bold()
                                                            .text_xs()
                                                            .text_color(cx.theme().primary),
                                                    ),
                                            )
                                            .child(
                                                Label::new(format!(
                                                    "{} items",
                                                    layout.root_node.children.len()
                                                ))
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground),
                                            ),
                                    )
                                    .children(node_elements),
                            )
                            // Floating Zoom & Fit Pill Overlay for Radial Canvas
                            .child(
                                h_flex()
                                    .id("float-zoom-pill")
                                    .absolute()
                                    .bottom(px(16.0))
                                    .right(px(16.0))
                                    .p_1()
                                    .gap_1()
                                    .rounded_full()
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .bg(cx.theme().background.opacity(0.92))
                                    .shadow_lg()
                                    .child(
                                        Button::new("btn-zoom-out")
                                            .icon(IconName::Minus)
                                            .ghost()
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.zoom_out(cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("btn-zoom-level")
                                            .label(format!("{:.0}%", zoom * 100.0))
                                            .ghost()
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.reset_zoom(cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("btn-zoom-in")
                                            .icon(IconName::Plus)
                                            .ghost()
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.zoom_in(cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("btn-zoom-fit")
                                            .icon(IconName::Maximize)
                                            .label("Fit")
                                            .ghost()
                                            .on_click(cx.listener({
                                                let l = layout_result.clone();
                                                move |this, _event, _window, cx| {
                                                    if let Some(layout) = &l {
                                                        this.fit_to_view(layout, cx);
                                                    }
                                                }
                                            })),
                                    )
                                    .child(
                                        Button::new("btn-pan-left")
                                            .icon(IconName::ChevronLeft)
                                            .ghost()
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.pan_by(60.0, 0.0, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("btn-pan-right")
                                            .icon(IconName::ChevronRight)
                                            .ghost()
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.pan_by(-60.0, 0.0, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("btn-pan-up")
                                            .icon(IconName::ChevronUp)
                                            .ghost()
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.pan_by(0.0, 60.0, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("btn-pan-down")
                                            .icon(IconName::ChevronDown)
                                            .ghost()
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.pan_by(0.0, -60.0, cx);
                                            })),
                                    ),
                            )
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
            })
    }
}

fn collect_radial_node_elements(
    node: &LayoutNode,
    selected_path: &Option<PathBuf>,
    expanded_paths: &std::collections::HashSet<PathBuf>,
    out: &mut Vec<AnyElement>,
    this: &GraphView,
    cx: &App,
) {
    for child in &node.children {
        let is_sel = selected_path.as_ref() == Some(&child.path);
        let is_exp = expanded_paths.contains(&child.path);
        out.push(
            this.render_compound_node(child, is_sel, is_exp, true, cx)
                .into_any_element(),
        );
        if is_exp && !child.children.is_empty() {
            collect_radial_node_elements(child, selected_path, expanded_paths, out, this, cx);
        }
    }
}

fn collect_edge_segments(
    parent_cx: f32,
    parent_cy: f32,
    parent_hw: f32,
    parent_hh: f32,
    children: &[LayoutNode],
    expanded_paths: &std::collections::HashSet<PathBuf>,
    zoom: f32,
    segments: &mut Vec<((f32, f32), (f32, f32))>,
) {
    for child in children {
        let child_w = (child.width * zoom).max(60.0);
        let child_h = (child.height * zoom).max(40.0);
        let child_cx = child.x * zoom + child_w / 2.0;
        let child_cy = child.y * zoom + child_h / 2.0;
        let child_hw = child_w / 2.0 + 1.0;
        let child_hh = child_h / 2.0 + 1.0;

        if let Some(seg) = clip_line_to_boxes(
            parent_cx, parent_cy, parent_hw, parent_hh,
            child_cx, child_cy, child_hw, child_hh,
        ) {
            segments.push(seg);
        }

        if expanded_paths.contains(&child.path) && !child.children.is_empty() {
            collect_edge_segments(
                child_cx,
                child_cy,
                child_hw,
                child_hh,
                &child.children,
                expanded_paths,
                zoom,
                segments,
            );
        }
    }
}

fn clip_line_to_boxes(
    cx1: f32,
    cy1: f32,
    hw1: f32,
    hh1: f32,
    cx2: f32,
    cy2: f32,
    hw2: f32,
    hh2: f32,
) -> Option<((f32, f32), (f32, f32))> {
    let dx = cx2 - cx1;
    let dy = cy2 - cy1;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist < 1.0 {
        return None;
    }

    let ux = dx / dist;
    let uy = dy / dist;

    let tx1 = if ux.abs() > 1e-6 {
        hw1 / ux.abs()
    } else {
        f32::INFINITY
    };
    let ty1 = if uy.abs() > 1e-6 {
        hh1 / uy.abs()
    } else {
        f32::INFINITY
    };
    let t1 = tx1.min(ty1);

    let tx2 = if ux.abs() > 1e-6 {
        hw2 / ux.abs()
    } else {
        f32::INFINITY
    };
    let ty2 = if uy.abs() > 1e-6 {
        hh2 / uy.abs()
    } else {
        f32::INFINITY
    };
    let t2 = tx2.min(ty2);

    if t1 + t2 >= dist {
        return None;
    }

    let start = (cx1 + ux * t1, cy1 + uy * t1);
    let end = (cx2 - ux * t2, cy2 - uy * t2);

    Some((start, end))
}
