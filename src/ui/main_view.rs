use std::path::PathBuf;
use gpui::prelude::*;
use gpui::{
    Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Window,
    div, px,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{ActiveTheme, Root, Theme, TitleBar, h_flex, v_flex};

use crate::settings_content::SettingsContent;
use crate::ui::{Breadcrumbs, FilesPanel, GraphView, InspectorPanel, StatusBar};
use crate::workspace::Workspace;

pub struct MainView {
    workspace: Entity<Workspace>,
    breadcrumbs: Entity<Breadcrumbs>,
    files_panel: Entity<FilesPanel>,
    graph_view: Entity<GraphView>,
    inspector_panel: Entity<InspectorPanel>,
    status_bar: Entity<StatusBar>,
    settings_content: Entity<SettingsContent>,
    show_settings: bool,
    show_left_sidebar: bool,
    show_right_sidebar: bool,
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

        let subscription = window.observe_window_appearance(|window, cx| {
            Theme::sync_system_appearance(Some(window), cx);
            cx.refresh_windows();
        });

        Self {
            workspace,
            breadcrumbs,
            files_panel,
            graph_view,
            inspector_panel,
            status_bar,
            settings_content,
            show_settings: false,
            show_left_sidebar: true,
            show_right_sidebar: true,
            _appearance_subscription: subscription,
        }
    }
}

impl Render for MainView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ws = self.workspace.clone();

        v_flex()
            .size_full()
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
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Button::new("toggle-left-panel")
                                    .label(if self.show_left_sidebar { "◀ Tree" } else { "▶ Tree" })
                                    .ghost()
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.show_left_sidebar = !this.show_left_sidebar;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("toggle-right-panel")
                                    .label(if self.show_right_sidebar { "Details ▶" } else { "Details ◀" })
                                    .ghost()
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.show_right_sidebar = !this.show_right_sidebar;
                                        cx.notify();
                                    })),
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
                    ),
            )
            .child(
                div().flex_1().size_full().child(if self.show_settings {
                    self.settings_content.clone().into_any_element()
                } else {
                    v_flex()
                        .size_full()
                        // Top Breadcrumbs Bar
                        .child(self.breadcrumbs.clone().into_any_element())
                        // Main 3-Pane Work Area
                        .child(
                            h_flex()
                                .flex_1()
                                .size_full()
                                // Left Files Tree Panel
                                .when(self.show_left_sidebar, |el| {
                                    el.child(
                                        div()
                                            .w(px(260.0))
                                            .h_full()
                                            .border_r_1()
                                            .border_color(cx.theme().border)
                                            .child(self.files_panel.clone().into_any_element()),
                                    )
                                })
                                // Center Graph Canvas
                                .child(
                                    div()
                                        .flex_1()
                                        .h_full()
                                        .child(self.graph_view.clone().into_any_element()),
                                )
                                // Right Details / Inspector Panel
                                .when(self.show_right_sidebar, |el| {
                                    el.child(
                                        div()
                                            .w(px(320.0))
                                            .h_full()
                                            .border_l_1()
                                            .border_color(cx.theme().border)
                                            .child(self.inspector_panel.clone().into_any_element()),
                                    )
                                }),
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
