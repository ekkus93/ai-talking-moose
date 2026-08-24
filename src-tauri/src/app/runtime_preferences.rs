use crate::app::state::AppSettings;
#[cfg(any(target_os = "macos", test))]
use std::path::Path;
use tauri::{Manager, Runtime};

#[cfg(any(target_os = "macos", test))]
const LOGIN_AGENT_LABEL: &str = "com.talkingmoose.ai";
#[cfg(any(target_os = "macos", test))]
const LOGIN_AGENT_FILENAME: &str = "com.talkingmoose.ai.plist";

#[cfg(any(target_os = "macos", test))]
fn plist_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(any(target_os = "macos", test))]
fn launch_agent_contents(executable: &Path) -> Result<String, String> {
    let executable = executable
        .to_str()
        .ok_or_else(|| "launch-at-login executable path is not valid UTF-8".to_string())?;
    let executable = plist_escape(executable);
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LOGIN_AGENT_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{executable}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>ProcessType</key>
    <string>Interactive</string>
    <key>LimitLoadToSessionType</key>
    <string>Aqua</string>
</dict>
</plist>
"#
    ))
}

#[cfg(any(target_os = "macos", test))]
fn sync_launch_agent_file(
    launch_agents_dir: &Path,
    executable: &Path,
    enabled: bool,
) -> Result<(), String> {
    let path = launch_agents_dir.join(LOGIN_AGENT_FILENAME);
    if !enabled {
        return match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("failed to disable launch at login: {error}")),
        };
    }

    std::fs::create_dir_all(launch_agents_dir)
        .map_err(|error| format!("failed to create LaunchAgents directory: {error}"))?;
    let contents = launch_agent_contents(executable)?;
    if std::fs::read_to_string(&path).ok().as_deref() == Some(contents.as_str()) {
        return Ok(());
    }

    let temporary_path = launch_agents_dir.join(format!(
        ".{LOGIN_AGENT_FILENAME}.tmp-{}",
        std::process::id()
    ));
    std::fs::write(&temporary_path, contents)
        .map_err(|error| format!("failed to stage launch-at-login configuration: {error}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temporary_path, std::fs::Permissions::from_mode(0o644))
            .map_err(|error| format!("failed to secure launch-at-login configuration: {error}"))?;
    }

    std::fs::rename(&temporary_path, &path)
        .map_err(|error| format!("failed to install launch-at-login configuration: {error}"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn sync_launch_at_login<R: Runtime>(
    app: &tauri::AppHandle<R>,
    enabled: bool,
) -> Result<(), String> {
    let home_dir = app.path().home_dir().map_err(|error| error.to_string())?;
    let executable = std::env::current_exe()
        .map_err(|error| format!("failed to locate Talking Moose executable: {error}"))?;
    sync_launch_agent_file(&home_dir.join("Library/LaunchAgents"), &executable, enabled)
}

#[cfg(not(target_os = "macos"))]
fn sync_launch_at_login<R: Runtime>(
    _app: &tauri::AppHandle<R>,
    _enabled: bool,
) -> Result<(), String> {
    Ok(())
}

fn set_always_on_top<R: Runtime>(app: &tauri::AppHandle<R>, enabled: bool) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main Moose window is unavailable".to_string())?;
    window
        .set_always_on_top(enabled)
        .map_err(|error| error.to_string())
}

fn set_tray_visible<R: Runtime>(app: &tauri::AppHandle<R>, visible: bool) -> Result<(), String> {
    let tray = app
        .tray_by_id(crate::app::tray::TRAY_ID)
        .ok_or_else(|| "Talking Moose tray icon is unavailable".to_string())?;
    tray.set_visible(visible).map_err(|error| error.to_string())
}

pub(crate) fn apply_startup_runtime_preferences<R: Runtime>(
    app: &tauri::AppHandle<R>,
    settings: &AppSettings,
) -> Result<(), String> {
    sync_launch_at_login(app, settings.launch_at_login)?;
    set_always_on_top(app, settings.always_on_top)
}

pub(crate) fn apply_changed_runtime_preferences<R: Runtime>(
    app: &tauri::AppHandle<R>,
    previous: &AppSettings,
    next: &AppSettings,
) -> Result<(), String> {
    let launch_changed = previous.launch_at_login != next.launch_at_login;
    let window_changed = previous.always_on_top != next.always_on_top;
    let tray_changed = previous.show_in_menu_bar != next.show_in_menu_bar;

    if launch_changed {
        sync_launch_at_login(app, next.launch_at_login)?;
    }

    if window_changed {
        if let Err(error) = set_always_on_top(app, next.always_on_top) {
            if launch_changed {
                let _ = sync_launch_at_login(app, previous.launch_at_login);
            }
            return Err(error);
        }
    }

    if tray_changed {
        if let Err(error) = set_tray_visible(app, next.show_in_menu_bar) {
            if window_changed {
                let _ = set_always_on_top(app, previous.always_on_top);
            }
            if launch_changed {
                let _ = sync_launch_at_login(app, previous.launch_at_login);
            }
            return Err(error);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_agent_round_trip_is_idempotent_and_removable() {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("Talking Moose & Friends");

        sync_launch_agent_file(directory.path(), &executable, true).unwrap();
        let path = directory.path().join(LOGIN_AGENT_FILENAME);
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains(LOGIN_AGENT_LABEL));
        assert!(contents.contains("Talking Moose &amp; Friends"));
        assert!(contents.contains("<key>RunAtLoad</key>"));

        sync_launch_agent_file(directory.path(), &executable, true).unwrap();
        assert!(path.is_file());

        sync_launch_agent_file(directory.path(), &executable, false).unwrap();
        assert!(!path.exists());
        sync_launch_agent_file(directory.path(), &executable, false).unwrap();
    }
}
