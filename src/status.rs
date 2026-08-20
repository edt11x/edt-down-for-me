//! Map last-success timestamps onto the traffic-light status colors.

use std::time::{Duration, Instant};

/// Grey: no successful probe yet.
pub const STATUS_GREY: i32 = 0;
/// Green: responded within the last 12 seconds.
pub const STATUS_GREEN: i32 = 1;
/// Yellow: last success was 12–60 seconds ago (stale, including the 36s window).
pub const STATUS_YELLOW: i32 = 2;
/// Red: no success in at least 60 seconds.
pub const STATUS_RED: i32 = 3;

pub const GREEN_WINDOW: Duration = Duration::from_secs(12);
pub const YELLOW_WINDOW: Duration = Duration::from_secs(36);
pub const RED_AFTER: Duration = Duration::from_secs(60);

const _: () = {
    assert!(GREEN_WINDOW.as_secs() < YELLOW_WINDOW.as_secs());
    assert!(YELLOW_WINDOW.as_secs() < RED_AFTER.as_secs());
};

/// Compute the indicator for a site.
///
/// - Grey at initialization (never succeeded, and 60s have not yet elapsed).
/// - Green if the latest success is younger than 12s.
/// - Yellow if the latest success is 12s or older, but still within 60s
///   (covers "responded within 36s" as a stale-but-alive window).
/// - Red if there has been no success for 60s (including never, after 60s).
pub fn status_for(now: Instant, added_at: Instant, last_ok: Option<Instant>) -> i32 {
    match last_ok {
        Some(t) => {
            let age = now.saturating_duration_since(t);
            if age < GREEN_WINDOW {
                STATUS_GREEN
            } else if age < RED_AFTER {
                STATUS_YELLOW
            } else {
                STATUS_RED
            }
        }
        None => {
            if now.saturating_duration_since(added_at) >= RED_AFTER {
                STATUS_RED
            } else {
                STATUS_GREY
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grey_at_start() {
        let t0 = Instant::now();
        assert_eq!(status_for(t0, t0, None), STATUS_GREY);
        assert_eq!(
            status_for(t0 + Duration::from_secs(59), t0, None),
            STATUS_GREY
        );
    }

    #[test]
    fn green_when_recent() {
        let t0 = Instant::now();
        let last = t0;
        assert_eq!(
            status_for(t0 + Duration::from_secs(11), t0, Some(last)),
            STATUS_GREEN
        );
    }

    #[test]
    fn yellow_after_twelve_including_thirty_six() {
        let t0 = Instant::now();
        let last = t0;
        assert_eq!(
            status_for(t0 + Duration::from_secs(12), t0, Some(last)),
            STATUS_YELLOW
        );
        assert_eq!(
            status_for(t0 + Duration::from_secs(36), t0, Some(last)),
            STATUS_YELLOW
        );
        assert_eq!(
            status_for(t0 + Duration::from_secs(59), t0, Some(last)),
            STATUS_YELLOW
        );
    }

    #[test]
    fn red_after_sixty() {
        let t0 = Instant::now();
        assert_eq!(
            status_for(t0 + Duration::from_secs(60), t0, Some(t0)),
            STATUS_RED
        );
        assert_eq!(
            status_for(t0 + Duration::from_secs(60), t0, None),
            STATUS_RED
        );
    }
}
