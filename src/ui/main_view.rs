use std::path::PathBuf;
use gpui::prelude::*;
use gpui::{
    Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Window,
    div, px,
};
use gpui_component::dock::{DockLayout, DockPlacement};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{Root, Theme, TitleBar, h_flex, v_flex};

use crate::settings_content::SettingsContent;
use crate::ui::{Breadcrumbs, FilesPanel, GraphView, InspectorPanel, StatusBar};
use crate::workspace::Workspace;

pub struct MainView {
    workspace: Entity<Workspace>,
    breadcrumbs: Entity<Breadcrumbs>,
    _files_panel: Entity<FilesPanel>,
    _graph_view: Entity<GraphView>,
    _inspector_panel: Entity<InspectorPanel>,
    status_bar: Entity<StatusBar>,
    settings_content: Entity<SettingsContent>,
    show_settings: bool,
    _appearance_subscription: Subscription,
}

impl MainView {
    pub fn new(workspace: Entity<Workspace>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let breadcrumbs = cx.new(|cx| Breadcrumbs::new(workspace.clone(), window, cx));
        let files_panel = cx.new(|cx| FilesPanel::new(workspace.clone(), window, cx));
        let graph_view = cx.new(|cx| GraphView::new(workspace.clone(), window, cx));
        let inspector_panel = cx.new(|cx| InspectorPanel::new(workspace.clone(), window, cx));
        let status_bar = cx.new(|cx| StatusBar::new(workspace.clone(), window, cx));
        let settings_content = cx.new(|cx| SettingsContent::new(window, cx));

        // Fully adopt gpui_component DockArea:
        // Center: GraphView
        // Left Dock: FilesPanel
        // Right Dock: InspectorPanel
        let dock_area = workspace.read(cx).dock_area.clone();
        let center_layout = DockLayout::tabs().panel(graph_view.clone());
        let left_layout = DockLayout::tabs().panel(files_panel.clone());
        let right_layout = DockLayout::tabs().panel(inspector_panel.clone());

        dock_area.update(cx, |dock, cx| {
            dock.set_center(center_layout, window, cx);
            dock.set_dock(DockPlacement::Left, left_layout, window, cx);
            dock.set_dock(DockPlacement::Right, right_layout, window, cx);
            dock.set_dock_size(DockPlacement::Left, px(260.0), window, cx);
            dock.set_dock_size(DockPlacement::Right, px(320.0), window, cx);
        });

        let subscription = window.observe_window_appearance(|window, cx| {
            Theme::sync_system_appearance(Some(window), cx);
            cx.refresh_windows();
        });

        Self {
            workspace,
            breadcrumbs,
            _files_panel: files_panel,
            _graph_view: graph_view,
            _inspector_panel: inspector_panel,
            status_bar,
            settings_content,
            show_settings: false,
            _appearance_subscription: subscription,
        }
    }
}

impl Render for MainView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ws = self.workspace.clone();
        let dock_area = self.workspace.read(cx).dock_area.clone();

        v_flex()
            .size_full()
            .overflow_hidden()
            .child(
                TitleBar::new()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child("🌌 Graph File Explorer")
                            .child(
                                Button::new("btn-open-workspace")
                                    .label("📂 Home Folder")
                                    .ghost()
                                    .on_click(cx.listener({
                                        let ws = ws.clone();
                                        move |_this, _event, _window, cx| {
                                            if let Ok(home) = std::env::var("HOME") {
                                                ws.update(cx, |ws, cx| {
                                                    ws.open_root(PathBuf::from(home), cx);
                                                });
                                            }
                                        }
                                    })),
                            ),
                    )
                    .child(
                        Button::new("settings-button")
                            .label(if self.show_settings {
                                "Back to Graph"
                            } else {
                                "⚙ Settings"
                            })
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                this.show_settings = !this.show_settings;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div().flex_1().min_h(px(0.0)).size_full().overflow_hidden().child(if self.show_settings {
                    self.settings_content.clone().into_any_element()
                } else {
                    v_flex()
                        .size_full()
                        .overflow_hidden()
                        // Top Breadcrumbs Bar
                        .child(self.breadcrumbs.clone().into_any_element())
                        // Native gpui_component DockArea (resizable splits, collapsible sidebars, draggable tabs)
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .min_h(px(0.0))
                                .size_full()
                                .overflow_hidden()
                                .child(dock_area.into_any_element()),
                        )
                        // Bottom Status Bar
                        .child(self.status_bar.clone().into_any_element())
                        .into_any_element()
                }),
            )
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
