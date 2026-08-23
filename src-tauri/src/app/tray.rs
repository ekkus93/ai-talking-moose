use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, Runtime};

pub(crate) const TRAY_ID: &str = "talking-moose-tray";

const ACTION_SHOW: &str = "tray-show";
const ACTION_HIDE: &str = "tray-hide";
const ACTION_START: &str = "tray-start";
const ACTION_STOP: &str = "tray-stop";
const ACTION_MUTE: &str = "tray-mute";
const ACTION_UNMUTE: &str = "tray-unmute";
const ACTION_SETTINGS: &str = "tray-settings";
const ACTION_QUIT: &str = "tray-quit";

fn show_main_window<R: Runtime>(app: &tauri::AppHandle<R>, focus: bool) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.show();
    if focus {
        let _ = window.set_focus();
    }
}

fn handle_menu_action<R: Runtime>(app: &tauri::AppHandle<R>, action: &str) {
    match action {
        ACTION_SHOW => show_main_window(app, true),
        ACTION_HIDE => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
        }
        ACTION_SETTINGS => {
            show_main_window(app, true);
            let _ = app.emit("moose://ui/open-settings", ());
        }
        ACTION_START => {
            let _ = app.emit("moose://tray/action", "start_conversation");
        }
        ACTION_STOP => {
            let _ = app.emit("moose://tray/action", "stop_conversation");
        }
        ACTION_MUTE => {
            let _ = app.emit("moose://tray/action", "mute");
        }
        ACTION_UNMUTE => {
            let _ = app.emit("moose://tray/action", "unmute");
        }
        ACTION_QUIT => app.exit(0),
        _ => {}
    }
}

pub(crate) fn install<R: Runtime>(app: &mut tauri::App<R>, visible: bool) -> tauri::Result<()> {
    let show = MenuItemBuilder::with_id(ACTION_SHOW, "Show Moose").build(app)?;
    let hide = MenuItemBuilder::with_id(ACTION_HIDE, "Hide Moose").build(app)?;
    let start = MenuItemBuilder::with_id(ACTION_START, "Start Conversation").build(app)?;
    let stop = MenuItemBuilder::with_id(ACTION_STOP, "Stop Conversation").build(app)?;
    let mute = MenuItemBuilder::with_id(ACTION_MUTE, "Mute Moose").build(app)?;
    let unmute = MenuItemBuilder::with_id(ACTION_UNMUTE, "Unmute Moose").build(app)?;
    let settings = MenuItemBuilder::with_id(ACTION_SETTINGS, "Open Settings").build(app)?;
    let quit = MenuItemBuilder::with_id(ACTION_QUIT, "Quit Talking Moose").build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&show, &hide])
        .separator()
        .items(&[&start, &stop])
        .items(&[&mute, &unmute])
        .separator()
        .item(&settings)
        .separator()
        .item(&quit)
        .build()?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("The Talking Moose")
        .on_menu_event(|app, event| handle_menu_action(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle(), true);
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    let tray = builder.build(app)?;
    tray.set_visible(visible)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_actions_are_explicit_and_bounded() {
        assert_eq!(
            [
                ACTION_SHOW,
                ACTION_HIDE,
                ACTION_START,
                ACTION_STOP,
                ACTION_MUTE,
                ACTION_UNMUTE,
                ACTION_SETTINGS,
                ACTION_QUIT,
            ]
            .len(),
            8
        );
        assert!(ACTION_QUIT.starts_with("tray-"));
    }
}
