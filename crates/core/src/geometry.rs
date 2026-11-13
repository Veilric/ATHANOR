use serde::{Deserialize, Serialize};
#[derive(Clone, Debug)]
pub struct WorkArea {
    pub id: String,
    /// Native desktop pixels. Only local offsets are multiplied by scale.
    pub origin: (i32, i32),
    pub logical_size: (f64, f64),
    pub scale: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedPlacement {
    pub monitor_id: String,
    pub offset: (f64, f64),
}
#[derive(Clone, Debug)]
pub struct PlacedPanel {
    pub monitor_id: String,
    pub position: (i32, i32),
    pub size: (f32, f32),
    pub offset: (f64, f64),
}
/// Prefer an explicit display; otherwise use the foreground window's center.
pub fn target_monitor<'a>(
    areas: &'a [WorkArea],
    preferred: Option<&str>,
    foreground: Option<(i32, i32)>,
) -> Option<&'a str> {
    preferred
        .and_then(|id| areas.iter().find(|area| area.id == id))
        .or_else(|| {
            foreground.and_then(|(x, y)| {
                areas.iter().find(|area| {
                    f64::from(x) >= f64::from(area.origin.0)
                        && f64::from(y) >= f64::from(area.origin.1)
                        && f64::from(x)
                            < f64::from(area.origin.0) + area.logical_size.0 * area.scale
                        && f64::from(y)
                            < f64::from(area.origin.1) + area.logical_size.1 * area.scale
                })
            })
        })
        .or(areas.first())
        .map(|area| area.id.as_str())
}
pub fn place_panel(
    saved: Option<&SavedPlacement>,
    areas: &[WorkArea],
    preferred: Option<&str>,
    size: (f64, f64),
) -> Option<PlacedPanel> {
    let area = saved
        .and_then(|s| areas.iter().find(|a| a.id == s.monitor_id))
        .or_else(|| preferred.and_then(|id| areas.iter().find(|a| a.id == id)))
        .or(areas.first())?;
    if !area.logical_size.0.is_finite()
        || !area.logical_size.1.is_finite()
        || area.logical_size.0 <= 0.0
        || area.logical_size.1 <= 0.0
    {
        return None;
    }
    let width = size.0.min(area.logical_size.0).max(1.0);
    let height = size.1.min(area.logical_size.1).max(1.0);
    let max_x = (area.logical_size.0 - width).max(0.0);
    let max_y = (area.logical_size.1 - height).max(0.0);
    let default = (
        (max_x - 24.0).max(0.0),
        (area.logical_size.1 * 0.15).min(max_y),
    );
    let offset = saved
        .filter(|s| s.monitor_id == area.id && s.offset.0.is_finite() && s.offset.1.is_finite())
        .map(|s| s.offset)
        .unwrap_or(default);
    let offset = (offset.0.clamp(0.0, max_x), offset.1.clamp(0.0, max_y));
    let scale = if area.scale.is_finite() && area.scale > 0.0 {
        area.scale
    } else {
        1.0
    };
    Some(PlacedPanel {
        monitor_id: area.id.clone(),
        position: (
            (f64::from(area.origin.0) + offset.0 * scale).round() as i32,
            (f64::from(area.origin.1) + offset.1 * scale).round() as i32,
        ),
        size: (width as f32, height as f32),
        offset,
    })
}
