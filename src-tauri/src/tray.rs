mod icon;

use std::{collections::HashSet, sync::Arc};

use chrono::Utc;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Wry,
};

use crate::{
    app_state::{AppState, AppViewState, ProviderStatus, ProviderView},
    config::Theme,
    providers::UsageWindow,
};

const PROVIDER_TRAY_PREFIX: &str = "provider-";
const FALLBACK_TRAY_ID: &str = "limitlens-control";

pub fn sync(app: &AppHandle, view: &AppViewState) -> tauri::Result<()> {
    let visible = view
        .providers
        .iter()
        .filter(|provider| provider.enabled && provider.detected)
        .collect::<Vec<_>>();
    let desired_ids = visible
        .iter()
        .map(|provider| provider_tray_id(&provider.id))
        .collect::<HashSet<_>>();

    for provider in &view.providers {
        let tray_id = provider_tray_id(&provider.id);
        if !desired_ids.contains(&tray_id) {
            let _ = app.remove_tray_by_id(&tray_id);
        }
    }

    if visible.is_empty() {
        upsert_fallback(app, view)?;
    } else {
        let _ = app.remove_tray_by_id(FALLBACK_TRAY_ID);
        for provider in visible {
            upsert_provider(app, view, provider)?;
        }
    }

    Ok(())
}

fn upsert_provider(
    app: &AppHandle,
    view: &AppViewState,
    provider: &ProviderView,
) -> tauri::Result<()> {
    let tray_id = provider_tray_id(&provider.id);
    let window = selected_window(provider);
    let used_percent = window.and_then(|window| window.used_percent);
    let image = icon::render_usage_icon(used_percent, view.theme, is_stale(&provider.status));
    let tooltip = provider_tooltip(provider, window);
    let menu = build_menu(app, view, Some(provider))?;

    if let Some(tray) = app.tray_by_id(&tray_id) {
        tray.set_icon(Some(image))?;
        tray.set_tooltip(Some(&tooltip))?;
        tray.set_menu(Some(menu))?;
        return Ok(());
    }

    TrayIconBuilder::with_id(tray_id)
        .icon(image)
        .tooltip(&tooltip)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(handle_tray_event)
        .build(app)?;
    Ok(())
}

fn upsert_fallback(app: &AppHandle, view: &AppViewState) -> tauri::Result<()> {
    let image = icon::render_usage_icon(None, view.theme, false);
    let menu = build_menu(app, view, None)?;
    let tooltip = "LimitLens - no enabled provider client was detected";

    if let Some(tray) = app.tray_by_id(FALLBACK_TRAY_ID) {
        tray.set_icon(Some(image))?;
        tray.set_tooltip(Some(tooltip))?;
        tray.set_menu(Some(menu))?;
        return Ok(());
    }

    TrayIconBuilder::with_id(FALLBACK_TRAY_ID)
        .icon(image)
        .tooltip(tooltip)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .build(app)?;
    Ok(())
}

fn build_menu(
    app: &AppHandle,
    view: &AppViewState,
    focused_provider: Option<&ProviderView>,
) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;
    if let Some(provider) = focused_provider {
        let summary = MenuItem::with_id(
            app,
            "usage-summary",
            provider_summary(provider),
            false,
            None::<&str>,
        )?;
        menu.append(&summary)?;
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }

    menu.append(&MenuItem::with_id(
        app,
        "refresh",
        if view.refreshing {
            "Refreshing..."
        } else {
            "Refresh Now"
        },
        !view.refreshing,
        None::<&str>,
    )?)?;

    let settings = Submenu::with_id(app, "settings", "Settings", true)?;
    let providers = Submenu::with_id(app, "settings-providers", "Providers", true)?;
    for provider in &view.providers {
        let label = format!(
            "{} - {}",
            provider.display_name,
            provider_status_label(provider)
        );
        providers.append(&CheckMenuItem::with_id(
            app,
            format!("toggle-provider:{}", provider.id),
            label,
            true,
            provider.enabled,
            None::<&str>,
        )?)?;
    }
    settings.append(&providers)?;

    let intervals = Submenu::with_id(app, "settings-interval", "Refresh Interval", true)?;
    for minutes in [1_u8, 2, 5, 10] {
        intervals.append(&CheckMenuItem::with_id(
            app,
            format!("interval:{minutes}"),
            format!("{minutes} minute{}", if minutes == 1 { "" } else { "s" }),
            true,
            view.refresh_interval_minutes == minutes,
            None::<&str>,
        )?)?;
    }
    settings.append(&intervals)?;

    let themes = Submenu::with_id(app, "settings-theme", "Theme", true)?;
    themes.append(&CheckMenuItem::with_id(
        app,
        "theme:dark",
        "Dark",
        true,
        view.theme == Theme::Dark,
        None::<&str>,
    )?)?;
    themes.append(&CheckMenuItem::with_id(
        app,
        "theme:light",
        "Light",
        true,
        view.theme == Theme::Light,
        None::<&str>,
    )?)?;
    settings.append(&themes)?;
    settings.append(&CheckMenuItem::with_id(
        app,
        "autostart",
        "Start with System",
        true,
        view.start_with_system,
        None::<&str>,
    )?)?;
    menu.append(&settings)?;

    let about = Submenu::with_id(app, "about", "About", true)?;
    about.append(&MenuItem::with_id(
        app,
        "about-version",
        format!("LimitLens {}", env!("CARGO_PKG_VERSION")),
        false,
        None::<&str>,
    )?)?;
    about.append(&MenuItem::with_id(
        app,
        "about-privacy",
        "Local-first - no telemetry",
        false,
        None::<&str>,
    )?)?;
    menu.append(&about)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?)?;
    Ok(menu)
}

fn handle_tray_event(tray: &tauri::tray::TrayIcon, event: TrayIconEvent) {
    if !matches!(
        event,
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        }
    ) {
        return;
    }
    let Some(provider_id) = tray.id().as_ref().strip_prefix(PROVIDER_TRAY_PREFIX) else {
        return;
    };
    let provider_id = provider_id.to_owned();
    let app = tray.app_handle().clone();
    let Some(state) = app.try_state::<Arc<AppState>>() else {
        return;
    };
    let state = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        let _ = state.cycle_provider_window(&app, &provider_id).await;
    });
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    handle_action(app, event.id().as_ref().to_owned());
}

pub(crate) fn handle_action(app: &AppHandle, action: String) {
    if action == "quit" {
        app.exit(0);
        return;
    }
    let Some(state) = app.try_state::<Arc<AppState>>() else {
        return;
    };
    let state = state.inner().clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if action == "refresh" {
            state.refresh(app).await;
        } else if action == "autostart" {
            let enabled = !state.view_state().await.start_with_system;
            if state.set_autostart(&app, enabled).await.is_ok() {
                state.sync_tray(&app).await;
            }
        } else if let Some(provider_id) = action.strip_prefix("toggle-provider:") {
            let enabled = state
                .view_state()
                .await
                .providers
                .iter()
                .find(|provider| provider.id == provider_id)
                .map(|provider| !provider.enabled)
                .unwrap_or(false);
            if state
                .set_provider_enabled(&app, provider_id, enabled)
                .await
                .is_ok()
                && enabled
            {
                state.refresh(app).await;
            }
        } else if let Some(minutes) = action
            .strip_prefix("interval:")
            .and_then(|value| value.parse::<u8>().ok())
        {
            let _ = state.set_refresh_interval(&app, minutes).await;
        } else if let Some(theme) = action.strip_prefix("theme:") {
            let theme = if theme == "light" {
                Theme::Light
            } else {
                Theme::Dark
            };
            state.set_theme(&app, theme).await;
        }
    });
}

fn provider_tray_id(provider_id: &str) -> String {
    format!("{PROVIDER_TRAY_PREFIX}{provider_id}")
}

fn selected_window(provider: &ProviderView) -> Option<&UsageWindow> {
    provider
        .selected_window_id
        .as_ref()
        .and_then(|selected| {
            provider
                .windows
                .iter()
                .find(|window| &window.id == selected)
        })
        .or_else(|| provider.windows.first())
}

fn provider_summary(provider: &ProviderView) -> String {
    let Some(window) = selected_window(provider) else {
        return format!("{} - Unavailable", provider.display_name);
    };
    let usage = window
        .used_percent
        .map(|percent| format!("{}% used", percent.round() as u8))
        .unwrap_or_else(|| "Unavailable".to_owned());
    format!(
        "{} - {} - {usage}",
        provider.display_name, window.display_name
    )
}

fn provider_tooltip(provider: &ProviderView, window: Option<&UsageWindow>) -> String {
    let mut parts = vec![provider.display_name.to_uppercase()];
    if let Some(window) = window {
        parts.push(window.display_name.clone());
        parts.push(
            window
                .used_percent
                .map(|percent| format!("{}% used", percent.round() as u8))
                .unwrap_or_else(|| "Usage unavailable".to_owned()),
        );
        if let Some(resets_at) = window.resets_at {
            let remaining = resets_at.signed_duration_since(Utc::now());
            if remaining.num_seconds() > 0 {
                let hours = remaining.num_hours();
                let minutes = remaining.num_minutes() % 60;
                parts.push(format!("resets in {hours}h {minutes}m"));
            }
        }
    } else {
        parts.push("Usage unavailable".to_owned());
    }
    parts.push(provider_status_label(provider));
    parts.join(" - ")
}

fn provider_status_label(provider: &ProviderView) -> String {
    if !provider.detected {
        return "Not detected".to_owned();
    }
    match provider.status {
        ProviderStatus::Loading => "Loading",
        ProviderStatus::Detected => "Detected",
        ProviderStatus::Live => "Live",
        ProviderStatus::Cached => "Cached",
        ProviderStatus::Stale => "Stale",
        ProviderStatus::AuthenticationRequired => "Authentication required",
        ProviderStatus::NotInstalled => "Not installed",
        ProviderStatus::Unsupported => "Usage unsupported",
        ProviderStatus::Error => "Error",
    }
    .to_owned()
}

fn is_stale(status: &ProviderStatus) -> bool {
    matches!(status, ProviderStatus::Cached | ProviderStatus::Stale)
}

#[cfg(test)]
mod tests {
    use super::provider_tray_id;

    #[test]
    fn provider_tray_ids_are_stable() {
        assert_eq!(provider_tray_id("codex"), "provider-codex");
    }
}
