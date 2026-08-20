use std::path::PathBuf;
use gpui::prelude::*;
use gpui::{
    Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Window,
    div, px,
};
use gpui_component::dock::{DockLayout, DockPlacement};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{ActiveTheme, Icon, IconName, Root, Sizable, StyledExt, Theme, TitleBar, h_flex, label::Label, v_flex};

use crate::settings_content::SettingsContent;
use crate::ui::{Breadcrumbs, FilesPanel, GraphView, InspectorPanel, StatusBar, TableView};
use crate::workspace::Workspace;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Graph,
    Table,
    Split,
}

pub struct MainView {
    workspace: Entity<Workspace>,
    breadcrumbs: Entity<Breadcrumbs>,
    _files_panel: Entity<FilesPanel>,
    graph_view: Entity<GraphView>,
    table_view: Entity<TableView>,
    _inspector_panel: Entity<InspectorPanel>,
    status_bar: Entity<StatusBar>,
    settings_content: Entity<SettingsContent>,
    view_mode: ViewMode,
    show_settings: bool,
    _appearance_subscription: Subscription,
}

impl MainView {
    pub fn new(workspace: Entity<Workspace>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let breadcrumbs = cx.new(|cx| Breadcrumbs::new(workspace.clone(), window, cx));
        let files_panel = cx.new(|cx| FilesPanel::new(workspace.clone(), window, cx));
        let graph_view = cx.new(|cx| GraphView::new(workspace.clone(), window, cx));
        let table_view = cx.new(|cx| TableView::new(workspace.clone(), window, cx));
        let inspector_panel = cx.new(|cx| InspectorPanel::new(workspace.clone(), window, cx));
        let status_bar = cx.new(|cx| StatusBar::new(workspace.clone(), window, cx));
        let settings_content = cx.new(|cx| SettingsContent::new(window, cx));

        // Configure DockArea with GraphView in center, FilesPanel in left dock, InspectorPanel in right dock
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
            graph_view,
            table_view,
            _inspector_panel: inspector_panel,
            status_bar,
            settings_content,
            view_mode: ViewMode::Graph,
            show_settings: false,
            _appearance_subscription: subscription,
        }
    }

    pub fn set_view_mode(&mut self, mode: ViewMode, window: &mut Window, cx: &mut Context<Self>) {
        if self.view_mode == mode {
            return;
        }
        self.view_mode = mode;

        let dock_area = self.workspace.read(cx).dock_area.clone();
        let center_layout = match mode {
            ViewMode::Graph => DockLayout::tabs().panel(self.graph_view.clone()),
            ViewMode::Table => DockLayout::tabs().panel(self.table_view.clone()),
            ViewMode::Split => DockLayout::h_split()
                .child(DockLayout::tabs().panel(self.graph_view.clone()), None)
                .child(DockLayout::tabs().panel(self.table_view.clone()), None),
        };

        dock_area.update(cx, |dock, cx| {
            dock.set_center(center_layout, window, cx);
        });

        cx.notify();
    }
}

impl Render for MainView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ws = self.workspace.clone();
        let dock_area = self.workspace.read(cx).dock_area.clone();
        let view_mode = self.view_mode;

        v_flex()
            .size_full()
            .overflow_hidden()
            .child(
                TitleBar::new()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(Icon::new(IconName::LayoutDashboard).small().text_color(cx.theme().primary))
                            .child(Label::new("Graph File Explorer").font_bold())
                            .child(
                                Button::new("btn-open-workspace")
                                    .icon(IconName::HardDrive)
                                    .label("Home")
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
                            .gap_1()
                            // View Mode Switcher
                            .child(
                                Button::new("btn-view-graph")
                                    .icon(IconName::LayoutDashboard)
                                    .label("Graph")
                                    .ghost()
                                    .when(view_mode == ViewMode::Graph, |b| b.font_bold().border_1())
                                    .on_click(cx.listener(|this, _event, window, cx| {
                                        this.set_view_mode(ViewMode::Graph, window, cx);
                                    })),
                            )
                            .child(
                                Button::new("btn-view-table")
                                    .icon(IconName::Frame)
                                    .label("Table")
                                    .ghost()
                                    .when(view_mode == ViewMode::Table, |b| b.font_bold().border_1())
                                    .on_click(cx.listener(|this, _event, window, cx| {
                                        this.set_view_mode(ViewMode::Table, window, cx);
                                    })),
                            )
                            .child(
                                Button::new("btn-view-split")
                                    .icon(IconName::PanelLeft)
                                    .label("Split")
                                    .ghost()
                                    .when(view_mode == ViewMode::Split, |b| b.font_bold().border_1())
                                    .on_click(cx.listener(|this, _event, window, cx| {
                                        this.set_view_mode(ViewMode::Split, window, cx);
                                    })),
                            )
                            .child(
                                Button::new("settings-button")
                                    .icon(IconName::Settings)
                                    .label(if self.show_settings {
                                        "Back"
                                    } else {
                                        "Settings"
                                    })
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.show_settings = !this.show_settings;
                                        cx.notify();
                                    })),
                            ),
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
