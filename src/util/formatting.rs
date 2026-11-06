use chrono::{DateTime, Datelike, Duration, Local, NaiveDate};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn title_case(input: &str) -> String {
    let mut chars = input.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Formats a message timestamp following the Keybase client convention:
///   - Today:            "4:34 PM"
///   - Yesterday:        "4:34 PM \u{2022} Yesterday"
///   - Within 7 days:    "4:34 PM \u{2022} Wed"
///   - Within 30 days:   "4:34 PM \u{2022} 5 Jan"
///   - Older:            "4:34 PM \u{2022} 5 Jan 24"
///
/// All spaces are replaced with non-breaking spaces (\u{00A0}) to prevent
/// line breaks inside the timestamp.
pub fn message_timestamp_label(timestamp_ms: Option<i64>) -> String {
    let Some(timestamp_ms) = timestamp_ms else {
        return "Now".to_string();
    };

    let Some(dt) = DateTime::from_timestamp_millis(timestamp_ms) else {
        return "Now".to_string();
    };
    let dt: DateTime<Local> = dt.with_timezone(&Local);
    let now = Local::now();

    let time_str = dt.format("%-I:%M %p").to_string();

    let label = if is_same_day(&dt, &now) {
        time_str
    } else if is_yesterday(&dt, &now) {
        format!("{time_str} \u{2022} Yesterday")
    } else if now.signed_duration_since(dt) < Duration::days(7) && dt < now {
        let day = dt.format("%a");
        format!("{time_str} \u{2022} {day}")
    } else if now.signed_duration_since(dt) < Duration::days(30) && dt < now {
        let date = dt.format("%-d %b");
        format!("{time_str} \u{2022} {date}")
    } else {
        let date = dt.format("%-d %b %y");
        format!("{time_str} \u{2022} {date}")
    };

    label.replace(' ', "\u{00A0}")
}

fn is_same_day(a: &DateTime<Local>, b: &DateTime<Local>) -> bool {
    a.year() == b.year() && a.ordinal() == b.ordinal()
}

fn is_yesterday(dt: &DateTime<Local>, now: &DateTime<Local>) -> bool {
    let yesterday = *now - Duration::days(1);
    is_same_day(dt, &yesterday)
}

pub fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

/// Local calendar day of a message timestamp, used to place day dividers.
pub fn local_day(timestamp_ms: i64) -> Option<NaiveDate> {
    DateTime::from_timestamp_millis(timestamp_ms).map(|dt| dt.with_timezone(&Local).date_naive())
}

/// Label for the divider that opens a day in the timeline: "Today", "Yesterday",
/// "Tuesday, Nov 4" within the current year, "Tuesday, Nov 4, 2025" before it.
pub fn day_divider_label(day: NaiveDate, today: NaiveDate) -> String {
    if day == today {
        "Today".to_string()
    } else if today.pred_opt() == Some(day) {
        "Yesterday".to_string()
    } else if day.year() == today.year() {
        day.format("%A, %b %-d").to_string()
    } else {
        day.format("%A, %b %-d, %Y").to_string()
    }
}

/// Short date for list rows such as search results: the time today,
/// "Yesterday", "Nov 4" within the year, "Nov 4, 2025" before it.
pub fn compact_date_label(timestamp_ms: Option<i64>) -> String {
    let Some(dt) = timestamp_ms.and_then(DateTime::from_timestamp_millis) else {
        return "Now".to_string();
    };
    let dt = dt.with_timezone(&Local);
    compact_date_label_for(dt.date_naive(), Local::now().date_naive(), &message_time_label(timestamp_ms))
}

fn compact_date_label_for(day: NaiveDate, today: NaiveDate, time: &str) -> String {
    if day == today {
        time.to_string()
    } else if today.pred_opt() == Some(day) {
        "Yesterday".to_string()
    } else if day.year() == today.year() {
        day.format("%b %-d").to_string()
    } else {
        day.format("%b %-d, %Y").to_string()
    }
}

/// Time of a sidebar row's newest message, as short as chat lists show it:
/// "9:41 AM" today, "Yesterday", the weekday within the past week ("Tue"),
/// "Nov 3" earlier this year, "11/3/25" before that.
pub fn sidebar_time_label(timestamp_ms: i64) -> Option<String> {
    let dt = DateTime::from_timestamp_millis(timestamp_ms)?.with_timezone(&Local);
    Some(sidebar_time_label_for(
        dt.date_naive(),
        Local::now().date_naive(),
        &message_time_label(Some(timestamp_ms)),
    ))
}

/// [`sidebar_time_label`] for a narrow sidebar: "9:41" today and the weekday
/// from yesterday on, so the time gives way before a name truncates.
pub fn sidebar_time_label_compact(timestamp_ms: i64) -> Option<String> {
    let dt = DateTime::from_timestamp_millis(timestamp_ms)?.with_timezone(&Local);
    Some(sidebar_time_label_compact_for(
        dt.date_naive(),
        Local::now().date_naive(),
        &dt.format("%-I:%M").to_string(),
    ))
}

fn sidebar_time_label_compact_for(day: NaiveDate, today: NaiveDate, clock: &str) -> String {
    if today.pred_opt() == Some(day) {
        return day.format("%a").to_string();
    }
    sidebar_time_label_for(day, today, clock)
}

fn sidebar_time_label_for(day: NaiveDate, today: NaiveDate, time: &str) -> String {
    if day >= today {
        // Today, or a clock running slightly ahead of this Mac's.
        time.to_string()
    } else if today.pred_opt() == Some(day) {
        "Yesterday".to_string()
    } else if today.signed_duration_since(day) < Duration::days(7) {
        day.format("%a").to_string()
    } else if day.year() == today.year() {
        day.format("%b %-d").to_string()
    } else {
        day.format("%-m/%-d/%y").to_string()
    }
}

/// Time of day only ("4:34 PM"), for rows that sit under a day divider.
pub fn message_time_label(timestamp_ms: Option<i64>) -> String {
    timestamp_ms
        .and_then(DateTime::from_timestamp_millis)
        .map(|dt| {
            dt.with_timezone(&Local)
                .format("%-I:%M %p")
                .to_string()
                .replace(' ', "\u{00A0}")
        })
        .unwrap_or_else(|| "Now".to_string())
}

/// Full date and time for a timestamp tooltip: "Thursday, November 5, 2026 at 6:10 PM".
pub fn message_full_timestamp(timestamp_ms: i64) -> Option<String> {
    DateTime::from_timestamp_millis(timestamp_ms).map(|dt| {
        dt.with_timezone(&Local)
            .format("%A, %B %-d, %Y at %-I:%M %p")
            .to_string()
    })
}

/// "Lena Fischer is typing…", "Lena Fischer and Sam Rivera are typing…",
/// "Several people are typing…". `names` are display names in arrival order.
pub fn typing_indicator_label<S: AsRef<str>>(names: &[S]) -> Option<String> {
    match names {
        [] => None,
        [one] => Some(format!("{} is typing…", one.as_ref())),
        [first, second] => Some(format!(
            "{} and {} are typing…",
            first.as_ref(),
            second.as_ref()
        )),
        _ => Some("Several people are typing…".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn compact_date_labels_shorten_with_age() {
        let today = date(2026, 11, 6);
        assert_eq!(compact_date_label_for(today, today, "10:15 AM"), "10:15 AM");
        assert_eq!(compact_date_label_for(date(2026, 11, 5), today, "x"), "Yesterday");
        assert_eq!(compact_date_label_for(date(2026, 11, 3), today, "x"), "Nov 3");
        assert_eq!(
            compact_date_label_for(date(2025, 12, 31), today, "x"),
            "Dec 31, 2025"
        );
    }

    #[test]
    fn sidebar_time_labels_shorten_with_age() {
        let today = date(2026, 11, 6); // a Friday
        let label = |day| sidebar_time_label_for(day, today, "9:41\u{00A0}AM");
        assert_eq!(label(today), "9:41\u{00A0}AM");
        assert_eq!(label(date(2026, 11, 7)), "9:41\u{00A0}AM");
        assert_eq!(label(date(2026, 11, 5)), "Yesterday");
        assert_eq!(label(date(2026, 11, 3)), "Tue");
        assert_eq!(label(date(2026, 10, 31)), "Sat");
        assert_eq!(label(date(2026, 10, 30)), "Oct 30");
        assert_eq!(label(date(2026, 1, 2)), "Jan 2");
        assert_eq!(label(date(2025, 12, 31)), "12/31/25");
        assert_eq!(sidebar_time_label(i64::MAX), None);
    }

    #[test]
    fn compact_sidebar_times_drop_the_period_and_name_yesterday_by_weekday() {
        let today = date(2026, 11, 6); // a Friday
        let label = |day| sidebar_time_label_compact_for(day, today, "10:30");
        assert_eq!(label(today), "10:30");
        assert_eq!(label(date(2026, 11, 5)), "Thu");
        assert_eq!(label(date(2026, 11, 3)), "Tue");
        assert_eq!(label(date(2026, 10, 30)), "Oct 30");
        assert_eq!(sidebar_time_label_compact(i64::MAX), None);
    }

    #[test]
    fn day_divider_labels_name_recent_days_and_spell_out_older_ones() {
        let today = date(2026, 11, 6);
        assert_eq!(day_divider_label(today, today), "Today");
        assert_eq!(day_divider_label(date(2026, 11, 5), today), "Yesterday");
        assert_eq!(
            day_divider_label(date(2026, 11, 3), today),
            "Tuesday, Nov 3"
        );
        assert_eq!(
            day_divider_label(date(2026, 1, 1), today),
            "Thursday, Jan 1"
        );
        assert_eq!(
            day_divider_label(date(2025, 12, 31), today),
            "Wednesday, Dec 31, 2025"
        );
        // Yesterday across a year boundary is still "Yesterday".
        assert_eq!(
            day_divider_label(date(2025, 12, 31), date(2026, 1, 1)),
            "Yesterday"
        );
    }

    #[test]
    fn local_day_follows_the_local_calendar() {
        let noon = Local
            .from_local_datetime(&date(2026, 3, 9).and_hms_opt(12, 0, 0).unwrap())
            .earliest()
            .unwrap();
        assert_eq!(local_day(noon.timestamp_millis()), Some(date(2026, 3, 9)));
        let before_midnight = noon + Duration::hours(11) + Duration::minutes(59);
        assert_eq!(
            local_day(before_midnight.timestamp_millis()),
            Some(date(2026, 3, 9))
        );
        let after_midnight = before_midnight + Duration::minutes(2);
        assert_eq!(
            local_day(after_midnight.timestamp_millis()),
            Some(date(2026, 3, 10))
        );
    }

    #[test]
    fn time_label_has_no_day_suffix_and_never_wraps() {
        let label = message_time_label(Some(
            Local
                .from_local_datetime(&date(2024, 2, 1).and_hms_opt(16, 34, 0).unwrap())
                .earliest()
                .unwrap()
                .timestamp_millis(),
        ));
        assert_eq!(label, "4:34\u{00A0}PM");
        assert_eq!(message_time_label(None), "Now");
    }

    #[test]
    fn typing_label_uses_names_and_collapses_crowds() {
        assert_eq!(typing_indicator_label::<&str>(&[]), None);
        assert_eq!(
            typing_indicator_label(&["Lena Fischer"]).as_deref(),
            Some("Lena Fischer is typing…")
        );
        assert_eq!(
            typing_indicator_label(&["Lena Fischer", "Sam Rivera"]).as_deref(),
            Some("Lena Fischer and Sam Rivera are typing…")
        );
        assert_eq!(
            typing_indicator_label(&["a", "b", "c"]).as_deref(),
            Some("Several people are typing…")
        );
    }
}
