use crate::persistence::Database;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

const WINDOW_POSITION_SETTING: &str = "moose_window_position";
const PERSIST_DEBOUNCE_MS: u64 = 250;
static PERSIST_GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DisplayBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub(crate) fn load_window_position(db: &Database) -> Result<Option<WindowPosition>, String> {
    let Some(json) = db
        .get_setting(WINDOW_POSITION_SETTING)
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };

    serde_json::from_str(&json)
        .map(Some)
        .map_err(|error| format!("stored Moose window position is invalid: {error}"))
}

pub(crate) fn persist_window_position(
    db: &Database,
    position: WindowPosition,
) -> Result<(), String> {
    let json = serde_json::to_string(&position).map_err(|error| error.to_string())?;
    db.set_setting(WINDOW_POSITION_SETTING, &json)
        .map_err(|error| error.to_string())
}

/// Debounce drag-time window move events so SQLite is not rewritten for every
/// intermediate pixel while still persisting the final settled position.
pub(crate) fn schedule_window_position_persist(db: Arc<Database>, position: WindowPosition) {
    let generation = PERSIST_GENERATION
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_add(1);

    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(PERSIST_DEBOUNCE_MS)).await;
        if PERSIST_GENERATION.load(Ordering::Relaxed) != generation {
            return;
        }

        if let Err(error) = persist_window_position(db.as_ref(), position) {
            warn!(error = %error, "Failed to persist Moose window position");
        }
    });
}

pub(crate) fn clamp_window_position(
    saved: WindowPosition,
    window_width: u32,
    window_height: u32,
    displays: &[DisplayBounds],
) -> Option<WindowPosition> {
    let target = displays
        .iter()
        .min_by_key(|display| squared_distance_to_display(saved, **display))?;

    let min_x = i64::from(target.x);
    let min_y = i64::from(target.y);
    let available_width = u64::from(target.width);
    let available_height = u64::from(target.height);
    let fitted_width = u64::from(window_width).min(available_width);
    let fitted_height = u64::from(window_height).min(available_height);
    let max_x = min_x + i64::try_from(available_width - fitted_width).unwrap_or(i64::MAX);
    let max_y = min_y + i64::try_from(available_height - fitted_height).unwrap_or(i64::MAX);

    Some(WindowPosition {
        x: i64::from(saved.x).clamp(min_x, max_x) as i32,
        y: i64::from(saved.y).clamp(min_y, max_y) as i32,
    })
}

fn squared_distance_to_display(position: WindowPosition, display: DisplayBounds) -> i128 {
    let min_x = i64::from(display.x);
    let min_y = i64::from(display.y);
    let max_x = min_x + i64::from(display.width.saturating_sub(1));
    let max_y = min_y + i64::from(display.height.saturating_sub(1));
    let x = i64::from(position.x);
    let y = i64::from(position.y);

    let dx = axis_distance(x, min_x, max_x);
    let dy = axis_distance(y, min_y, max_y);
    i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy)
}

fn axis_distance(value: i64, min: i64, max: i64) -> i64 {
    if value < min {
        min - value
    } else if value > max {
        value - max
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN_DISPLAY: DisplayBounds = DisplayBounds {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };

    #[test]
    fn persisted_position_round_trips_as_non_secret_setting() {
        let db = Database::new_in_memory().unwrap();
        let position = WindowPosition { x: 321, y: 654 };

        persist_window_position(&db, position).unwrap();

        assert_eq!(load_window_position(&db).unwrap(), Some(position));
    }

    #[test]
    fn visible_position_is_preserved() {
        assert_eq!(
            clamp_window_position(WindowPosition { x: 500, y: 300 }, 340, 450, &[MAIN_DISPLAY],),
            Some(WindowPosition { x: 500, y: 300 })
        );
    }

    #[test]
    fn disconnected_display_position_clamps_to_nearest_connected_display() {
        assert_eq!(
            clamp_window_position(
                WindowPosition { x: 4000, y: 900 },
                340,
                450,
                &[MAIN_DISPLAY],
            ),
            Some(WindowPosition { x: 1580, y: 630 })
        );
    }

    #[test]
    fn negative_coordinate_secondary_display_is_supported() {
        let left_display = DisplayBounds {
            x: -1280,
            y: 0,
            width: 1280,
            height: 1024,
        };

        assert_eq!(
            clamp_window_position(
                WindowPosition { x: -900, y: 200 },
                340,
                450,
                &[MAIN_DISPLAY, left_display],
            ),
            Some(WindowPosition { x: -900, y: 200 })
        );
    }

    #[test]
    fn window_larger_than_display_anchors_to_display_origin() {
        let small_display = DisplayBounds {
            x: 100,
            y: 200,
            width: 300,
            height: 200,
        };

        assert_eq!(
            clamp_window_position(
                WindowPosition { x: 999, y: 999 },
                800,
                600,
                &[small_display],
            ),
            Some(WindowPosition { x: 100, y: 200 })
        );
    }

    #[test]
    fn no_display_information_skips_restore() {
        assert_eq!(
            clamp_window_position(WindowPosition { x: 10, y: 20 }, 340, 450, &[]),
            None
        );
    }
}
