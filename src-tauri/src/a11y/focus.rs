//! Documented cockpit focus order contract (left → chrome → timeline → composer → right).

use serde::{Deserialize, Serialize};

/// Logical focus regions matching the closed cockpit IA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FocusRegion {
    LeftRail,
    TopChrome,
    Timeline,
    Composer,
    RightPane,
    Palette,
    Advanced,
}

impl FocusRegion {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LeftRail => "left_rail",
            Self::TopChrome => "top_chrome",
            Self::Timeline => "timeline",
            Self::Composer => "composer",
            Self::RightPane => "right_pane",
            Self::Palette => "palette",
            Self::Advanced => "advanced",
        }
    }

    /// CSS/data-region markers expected in shipped HTML.
    pub fn data_region(self) -> &'static str {
        match self {
            Self::LeftRail => "left",
            Self::TopChrome => "top-chrome",
            Self::Timeline => "center",
            Self::Composer => "composer",
            Self::RightPane => "right",
            Self::Palette => "palette",
            Self::Advanced => "advanced-settings",
        }
    }
}

/// Shipped primary focus order for keyboard-only journeys (palette/advanced are overlays).
pub const COCKPIT_FOCUS_ORDER: &[FocusRegion] = &[
    FocusRegion::LeftRail,
    FocusRegion::TopChrome,
    FocusRegion::Timeline,
    FocusRegion::Composer,
    FocusRegion::RightPane,
];

/// Return the focus order table for tests and UI documentation.
pub fn focus_order_regions() -> Vec<&'static str> {
    COCKPIT_FOCUS_ORDER
        .iter()
        .map(|r| r.as_str())
        .collect()
}

/// Next region in the primary cycle (does not enter overlays).
pub fn next_focus_region(current: FocusRegion) -> FocusRegion {
    let order = COCKPIT_FOCUS_ORDER;
    let idx = order.iter().position(|r| *r == current).unwrap_or(0);
    order[(idx + 1) % order.len()]
}

/// Previous region in the primary cycle.
pub fn prev_focus_region(current: FocusRegion) -> FocusRegion {
    let order = COCKPIT_FOCUS_ORDER;
    let idx = order.iter().position(|r| *r == current).unwrap_or(0);
    if idx == 0 {
        order[order.len() - 1]
    } else {
        order[idx - 1]
    }
}

/// Palette/advanced Esc must close without trapping (policy helper).
pub fn escape_closes_overlay(overlay_open: bool) -> bool {
    overlay_open // Esc is the documented close path when overlay is open
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_order_is_left_chrome_timeline_composer_right() {
        let order = focus_order_regions();
        assert_eq!(
            order,
            vec![
                "left_rail",
                "top_chrome",
                "timeline",
                "composer",
                "right_pane"
            ]
        );
        assert_eq!(
            next_focus_region(FocusRegion::LeftRail),
            FocusRegion::TopChrome
        );
        assert_eq!(
            next_focus_region(FocusRegion::RightPane),
            FocusRegion::LeftRail
        );
        assert!(escape_closes_overlay(true));
        assert!(!escape_closes_overlay(false));
    }
}
