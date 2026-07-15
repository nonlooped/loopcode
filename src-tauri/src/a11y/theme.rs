//! Theme preference (default + high-contrast).

use serde::{Deserialize, Serialize};

pub const THEME_DEFAULT: &str = "default";
pub const THEME_HIGH_CONTRAST: &str = "high_contrast";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    Default,
    HighContrast,
}

impl ThemePreference {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => THEME_DEFAULT,
            Self::HighContrast => THEME_HIGH_CONTRAST,
        }
    }

    /// CSS class applied on documentElement.
    pub fn css_class(self) -> &'static str {
        match self {
            Self::Default => "theme-default",
            Self::HighContrast => "theme-high-contrast",
        }
    }
}

pub fn default_theme() -> ThemePreference {
    ThemePreference::Default
}

pub fn parse_theme(s: &str) -> Option<ThemePreference> {
    match s {
        "default" | "dark" | "theme-default" => Some(ThemePreference::Default),
        "high_contrast" | "high-contrast" | "hc" | "theme-high-contrast" => {
            Some(ThemePreference::HighContrast)
        }
        _ => None,
    }
}

/// Pure apply helper: returns class list / data attribute for the document root.
pub fn apply_theme_preference(theme: ThemePreference) -> ThemeApplyResult {
    ThemeApplyResult {
        theme: theme.as_str().into(),
        css_class: theme.css_class().into(),
        data_theme: theme.as_str().into(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeApplyResult {
    pub theme: String,
    pub css_class: String,
    pub data_theme: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn themes_parse_and_apply() {
        assert_eq!(parse_theme("high_contrast"), Some(ThemePreference::HighContrast));
        let a = apply_theme_preference(ThemePreference::HighContrast);
        assert_eq!(a.css_class, "theme-high-contrast");
        assert_eq!(default_theme().as_str(), THEME_DEFAULT);
    }
}
