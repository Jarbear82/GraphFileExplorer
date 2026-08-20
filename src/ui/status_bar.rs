use gpui::{
    Context, Entity, IntoElement, ParentElement, Render, Styled, Window,
    px,
};
use gpui_component::{
    ActiveTheme, h_flex, label::Label,
};

use crate::workspace::Workspace;

pub struct StatusBar {
    workspace: Entity<Workspace>,
}

impl StatusBar {
    pub fn new(workspace: Entity<Workspace>, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self { workspace }
    }
}

impl Render for StatusBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ws = self.workspace.read(cx);

        let item_count = ws.current_entry.as_ref().map(|e| e.item_count).unwrap_or(0);
        let status_text = ws.status_message.clone().unwrap_or_default();
        let layout_name = ws.layout_kind.name();

        let metrics_text = if let Some(layout) = &ws.layout_result {
            format!(
                "Scale: {:.2}..{:.2} | Nodes: {} | Layout: {:.2}ms",
                layout.min_scale, layout.max_scale, layout.node_count, layout.compute_time_ms
            )
        } else {
            "Ready".to_string()
        };

        let selected_text = if let Some(sel) = &ws.selected_path {
            sel.file_name()
                .map(|n| format!("Selected: {}", n.to_string_lossy()))
                .unwrap_or_default()
        } else {
            format!("{item_count} items")
        };

        h_flex()
            .w_full()
            .h(px(26.0))
            .px_3()
            .gap_4()
            .items_center()
            .justify_between()
            .bg(cx.theme().secondary.opacity(0.5))
            .border_t_1()
            .border_color(cx.theme().border)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(Label::new(status_text).text_color(cx.theme().foreground))
                    .child(Label::new("|").text_color(cx.theme().border))
                    .child(Label::new(selected_text)),
            )
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(Label::new(format!("Algorithm: {layout_name}")))
                    .child(Label::new("|").text_color(cx.theme().border))
                    .child(Label::new(metrics_text)),
            )
    }
}
