pub fn configure(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    configure_platform(builder)
}

#[cfg(not(target_os = "windows"))]
fn configure_platform(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder
}

#[cfg(target_os = "windows")]
fn configure_platform(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    use tauri::WindowEvent;

    builder
        .setup(|app| {
            windows::install_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if should_hide_on_close(window.label()) {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Err(error) = window.hide() {
                        eprintln!("failed to hide Aster main window: {error}");
                    }
                }
            }
        })
}

#[cfg(any(target_os = "windows", test))]
fn should_hide_on_close(window_label: &str) -> bool {
    window_label == "main"
}

#[cfg(any(target_os = "windows", test))]
mod windows {
    use tauri::menu::MenuBuilder;
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use tauri::{App, AppHandle, Manager};

    const SHOW_MENU_ID: &str = "tray-show-main";
    const QUIT_MENU_ID: &str = "tray-quit";

    pub(super) fn install_tray(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
        let menu = MenuBuilder::new(app)
            .text(SHOW_MENU_ID, "显示 Aster")
            .separator()
            .text(QUIT_MENU_ID, "退出")
            .build()?;
        let icon = app
            .default_window_icon()
            .cloned()
            .ok_or_else(|| std::io::Error::other("Aster tray icon is unavailable"))?;
        TrayIconBuilder::with_id("aster-main-tray")
            .icon(icon)
            .menu(&menu)
            .show_menu_on_left_click(false)
            .tooltip("Aster 酒店物资运营管理")
            .on_menu_event(|app, event| match event.id().as_ref() {
                SHOW_MENU_ID => show_main_window(app),
                QUIT_MENU_ID => app.exit(0),
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if matches!(
                    event,
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
                ) {
                    show_main_window(tray.app_handle());
                }
            })
            .build(app)?;
        Ok(())
    }

    fn show_main_window(app: &AppHandle) {
        let Some(window) = app.get_webview_window("main") else {
            eprintln!("Aster main window is unavailable");
            return;
        };
        if let Err(error) = window
            .show()
            .and_then(|_| window.unminimize())
            .and_then(|_| window.set_focus())
        {
            eprintln!("failed to show Aster main window: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::should_hide_on_close;

    #[test]
    fn only_main_window_is_hidden_on_close() {
        assert!(should_hide_on_close("main"));
        assert!(!should_hide_on_close("editor-item-create"));
        assert!(!should_hide_on_close("connection-wizard"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn windows_tray_installer_is_type_checked_on_other_test_platforms() {
        let _installer = super::windows::install_tray;
    }
}
