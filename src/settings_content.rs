use gpui::{
    App, Context, Global, IntoElement, ParentElement, Render, SharedString, Styled, Window,
};
use gpui_component::{
    ActiveTheme, Sizable, Size, Theme, ThemeMode,
    group_box::GroupBoxVariant,
    setting::{NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage, Settings},
    v_flex,
};

/// Simple app-level settings store for values that aren't on Theme.
#[derive(Clone)]
pub struct AppSettings {
    pub font_family: SharedString,
    pub font_size: f64,
    pub auto_update: bool,
    pub confirm_before_delete: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            font_family: "Arial".into(),
            font_size: 14.0,
            auto_update: true,
            confirm_before_delete: true,
        }
    }
}

impl Global for AppSettings {}

pub fn app_settings(cx: &App) -> &AppSettings {
    if cx.has_global::<AppSettings>() {
        cx.global::<AppSettings>()
    } else {
        // Fallback if init was skipped; prefer initializing in main / SettingsContent::new
        unreachable!("AppSettings global not initialized")
    }
}

pub fn app_settings_mut(cx: &mut App) -> &mut AppSettings {
    cx.global_mut::<AppSettings>()
}

pub fn should_confirm_delete(cx: &App) -> bool {
    if cx.has_global::<AppSettings>() {
        cx.global::<AppSettings>().confirm_before_delete
    } else {
        true
    }
}

pub struct SettingsContent;

impl SettingsContent {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Ensure the global exists once.
        if !cx.has_global::<AppSettings>() {
            cx.set_global(AppSettings::default());
        }
        Self
    }
}

impl Render for SettingsContent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().size_full().bg(cx.theme().background).child(
            Settings::new("app-settings")
                .with_size(Size::Medium)
                .with_group_variant(GroupBoxVariant::Outline)
                .pages(vec![
                    SettingPage::new("General")
                        .resettable(true)
                        .default_open(true)
                        .groups(vec![
                            SettingGroup::new().title("Appearance").items(vec![
                                SettingItem::new(
                                    "Dark Mode",
                                    SettingField::switch(
                                        |cx: &App| cx.theme().is_dark(),
                                        |enabled: bool, cx: &mut App| {
                                            let mode = if enabled {
                                                ThemeMode::Dark
                                            } else {
                                                ThemeMode::Light
                                            };
                                            Theme::change(mode, None, cx);
                                            cx.refresh_windows();
                                        },
                                    )
                                    .default_value(false),
                                )
                                .description("Switch between light and dark themes."),
                            ]),
                            SettingGroup::new().title("Font").items(vec![
                                SettingItem::new(
                                    "Font Family",
                                    SettingField::dropdown(
                                        vec![
                                            ("Arial".into(), "Arial".into()),
                                            ("Helvetica".into(), "Helvetica".into()),
                                            ("System".into(), ".SystemUIFont".into()),
                                        ],
                                        |cx: &App| app_settings(cx).font_family.clone(),
                                        |val: SharedString, cx: &mut App| {
                                            app_settings_mut(cx).font_family = val;
                                            cx.refresh_windows();
                                        },
                                    )
                                    .default_value(SharedString::from("Arial")),
                                ),
                                SettingItem::new(
                                    "Font Size",
                                    SettingField::number_input(
                                        NumberFieldOptions {
                                            min: 8.0,
                                            max: 72.0,
                                            step: 1.0,
                                        },
                                        |cx: &App| app_settings(cx).font_size,
                                        |val: f64, cx: &mut App| {
                                            app_settings_mut(cx).font_size = val;
                                            cx.refresh_windows();
                                        },
                                    )
                                    .default_value(14.0),
                                ),
                            ]),
                            SettingGroup::new().title("Behavior").items(vec![
                                SettingItem::new(
                                    "Confirm Before Delete",
                                    SettingField::switch(
                                        |cx: &App| app_settings(cx).confirm_before_delete,
                                        |val: bool, cx: &mut App| {
                                            app_settings_mut(cx).confirm_before_delete = val;
                                        },
                                    )
                                    .default_value(true),
                                )
                                .description("Show a confirmation prompt before deleting fields, hubs, instances, or graphs."),
                            ]),
                        ]),
                    SettingPage::new("Software Update").resettable(true).group(
                        SettingGroup::new().title("Updates").items(vec![
                            SettingItem::new(
                                "Auto Update",
                                SettingField::switch(
                                    |cx: &App| app_settings(cx).auto_update,
                                    |val: bool, cx: &mut App| {
                                        app_settings_mut(cx).auto_update = val;
                                    },
                                )
                                .default_value(true),
                            )
                            .description("Automatically download and install updates."),
                        ]),
                    ),
                ]),
        )
    }
}
