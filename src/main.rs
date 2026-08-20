use gpui::{App, AppContext, WindowOptions};
use gpui_component::{Root, Theme, ThemeRegistry};
use std::path::PathBuf;

pub mod model;
pub mod settings_content;
pub mod ui;
pub mod workspace;

use crate::ui::MainView;
use crate::workspace::Workspace;

fn main() {
    let initial_path = std::env::args().nth(1).map(PathBuf::from);
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |app| {
        gpui_component::init(app);
        load_and_watch_themes(app);

        let initial_path_clone = initial_path.clone();
        app.spawn(async move |app| {
            app.open_window(WindowOptions::default(), |window, cx| {
                let initial = initial_path_clone.clone();
                let workspace = cx.new(|cx| Workspace::new(initial, window, cx));
                let view = cx.new(|cx| MainView::new(workspace, window, cx));
                // First-level child of the window must be Root
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}

fn load_and_watch_themes(cx: &mut App) {
    let themes_dir = PathBuf::from("./themes");
    if !themes_dir.exists() {
        let _ = std::fs::create_dir_all(&themes_dir);
    }

    // Load + watch. Closure runs after initial load and on every change.
    if let Err(err) = ThemeRegistry::watch_dir(themes_dir, cx, move |cx| {
        let (light, dark) = {
            let registry = ThemeRegistry::global(cx);
            (
                registry.themes().get("Molokai Light").cloned(),
                registry.themes().get("Molokai Dark").cloned(),
            )
        };

        if let Some(light) = light {
            Theme::global_mut(cx).light_theme = light;
        }
        if let Some(dark) = dark {
            Theme::global_mut(cx).dark_theme = dark;
        }

        Theme::sync_system_appearance(None, cx);
        cx.refresh_windows();
    }) {
        eprintln!("failed to bind themes file monitor: {:?}", err);
    }
}
