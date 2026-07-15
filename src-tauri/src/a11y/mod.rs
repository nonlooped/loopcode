//! Accessibility contracts, single-instance, notification redaction.

pub mod focus;
pub mod notifications;
pub mod single_instance;
pub mod theme;

pub use focus::{focus_order_regions, FocusRegion, COCKPIT_FOCUS_ORDER};
pub use notifications::{
    build_approval_notification, ApprovalNotificationPayload, NotificationDto,
};
pub use single_instance::{
    acquire_single_instance, release_single_instance, SingleInstanceDecision, SingleInstanceGuard,
};
pub use theme::{
    apply_theme_preference, default_theme, parse_theme, ThemePreference, THEME_HIGH_CONTRAST,
    THEME_DEFAULT,
};
