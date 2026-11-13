use crate::{cancel::Cancellation, feature::*, query::*};
use chrono::{Days, NaiveDate};

#[derive(Default)]
pub struct CalendarFeature;
impl CalendarFeature {
    pub fn new() -> Self {
        Self
    }
}
impl Feature for CalendarFeature {
    fn id(&self) -> &'static str {
        "calendar"
    }
    fn query(
        &self,
        request: &QueryRequest,
        context: &QueryContext,
        cancel: &Cancellation,
    ) -> FeatureReply {
        if cancel.is_cancelled() {
            return FeatureReply::NotApplicable;
        }
        let q = request.text.trim().to_ascii_lowercase();
        if let Some(dates) = q.strip_prefix("days between ") {
            let Some((a, b)) = dates.split_once(" and ") else {
                return FeatureReply::Incomplete;
            };
            let (Some(a), Some(b)) = (parse_date(a), parse_date(b)) else {
                return invalid("Use valid ISO dates: YYYY-MM-DD");
            };
            let days = b.signed_duration_since(a).num_days();
            return FeatureReply::Results(vec![ResultItem::answer(
                self.id(),
                q.clone(),
                format!("{days} days"),
                format!("{a} → {b} · Calendar days"),
                days.to_string(),
            )]);
        }
        let (base, remainder) = if let Some(rest) = q.strip_prefix("today") {
            if !rest.is_empty() && !rest.starts_with([' ', '+', '-']) {
                return FeatureReply::NotApplicable;
            }
            (context.today, rest.trim())
        } else if q.get(..10).is_some_and(looks_like_date) {
            let Some(date) = parse_date(&q[..10]) else {
                return invalid("That calendar date does not exist");
            };
            (date, q[10..].trim())
        } else {
            return FeatureReply::NotApplicable;
        };
        let result = if remainder.is_empty() {
            Some(base)
        } else {
            let parts: Vec<_> = remainder.split_whitespace().collect();
            if parts.len() < 3 {
                return FeatureReply::Incomplete;
            }
            if parts.len() != 3
                || !["day", "days"].contains(&parts[2])
                || !["+", "-"].contains(&parts[0])
            {
                return invalid("Use a date followed by + or - and a number of days");
            }
            let Ok(count) = parts[1].parse::<u64>() else {
                return invalid("Use a nonnegative whole number of days");
            };
            if parts[0] == "+" {
                base.checked_add_days(Days::new(count))
            } else {
                base.checked_sub_days(Days::new(count))
            }
        };
        let Some(result) = result else {
            return invalid("The resulting date is outside the supported calendar");
        };
        FeatureReply::Results(vec![ResultItem::answer(
            self.id(),
            q,
            result.format("%b %-d, %Y").to_string(),
            format!("{} · Calendar days", result.format("%A")),
            result.to_string(),
        )])
    }
}
fn looks_like_date(s: &str) -> bool {
    s.len() == 10
        && s.as_bytes().iter().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                *b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
}
fn parse_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    if !looks_like_date(s) {
        return None;
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}
fn invalid(s: &str) -> FeatureReply {
    FeatureReply::Invalid { message: s.into() }
}
