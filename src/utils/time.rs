//! Time utilities for consistent time range calculations across all providers

use crate::types::TimeRange;
use chrono::{Datelike, Local, TimeZone, Utc};

/// Get time ranges for today, this week, and this month using the user's local timezone.
/// This ensures that "today" matches the user's local day boundaries, not UTC.
///
/// Returns: (today_range, this_week_range, this_month_range)
pub fn get_local_time_ranges() -> (TimeRange, TimeRange, TimeRange) {
    // Use local timezone for calculating "today", "this week", "this month"
    // This ensures the user's local day boundaries are respected
    let now_local = Local::now();
    let now_utc = Utc::now();

    // Today's start in local timezone, converted to UTC for comparison
    let today_start_local = Local
        .with_ymd_and_hms(now_local.year(), now_local.month(), now_local.day(), 0, 0, 0)
        .unwrap();
    let today_start = today_start_local.with_timezone(&Utc);

    // Week start (Sunday) in local timezone, converted to UTC
    let week_start_local = today_start_local
        - chrono::Duration::days(now_local.weekday().num_days_from_sunday() as i64);
    let week_start = week_start_local.with_timezone(&Utc);

    // Month start in local timezone, converted to UTC
    let month_start_local = Local
        .with_ymd_and_hms(now_local.year(), now_local.month(), 1, 0, 0, 0)
        .unwrap();
    let month_start = month_start_local.with_timezone(&Utc);

    (
        TimeRange {
            start: today_start,
            end: now_utc,
        },
        TimeRange {
            start: week_start,
            end: now_utc,
        },
        TimeRange {
            start: month_start,
            end: now_utc,
        },
    )
}

