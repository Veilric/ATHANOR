use crate::{cancel::Cancellation, feature::*, query::*};
use std::sync::Arc;

pub struct HelpTopic {
    pub command: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub syntax: &'static str,
    pub examples: &'static [&'static str],
    pub limitations: &'static str,
}

pub const TOPICS: &[HelpTopic] = &[
    HelpTopic {
        command: "calculator",
        title: "Calculator",
        description: "Evaluate arithmetic and scientific expressions on your device.",
        syntax: "+ - * / % ^ and parentheses\nsqrt(x), abs(x), exp(x), ln(x), log(x)\nsin(x), cos(x), tan(x), asin(x), acos(x), atan(x), atan2(y,x)\nsinh(x), cosh(x), tanh(x), asinh(x), acosh(x), atanh(x)\nfloor(x), ceil(x), round(x), signum(x), min(a,b), max(a,b)\nConstants pi and e",
        examples: &["1+1", "sqrt(144)+2^5"],
        limitations: "Angle mode is selected in Settings. log is base 10 and ln is natural log. Arithmetic uses f64 precision with 12 significant display digits. A standalone number remains a search query. Use decimal points. Symbolic algebra, complex numbers and graphing are outside the supported grammar.",
    },
    HelpTopic {
        command: "units",
        title: "Measurement conversions",
        description: "Convert length, mass, temperature, area, volume, speed, duration and storage.",
        syntax: "<number> <unit> in <unit>\n<number> <unit> to <unit>\nLength m, mm, µm, cm, km, in, ft, yd, mi, nmi\nMass kg, g, mg, t, lb, oz, st\nTemperature C, F, K\nArea m2, cm2, km2, ft2, ha, ac\nVolume m3, l, ml, usgal, impgal\nSpeed m/s, km/h, mph, kn, ft/s\nDuration s, ms, min, h, days, wk\nStorage b, B, kb, kB, Mb, MB, Gb, GB, Tb, TB, Kib, KiB, Mib, MiB, Gib, GiB, Tib, TiB",
        examples: &["12 km in mi", "32 F to C", "1 MiB to KiB"],
        limitations: "Use a decimal point. Source and destination units must have the same dimension. Storage symbols distinguish bits and bytes by case. Decimal storage uses powers of 1000 and binary storage uses powers of 1024. Unspecified gallons offer US and imperial choices. Temperatures below absolute zero are rejected.",
    },
    HelpTopic {
        command: "calendar",
        title: "Calendar",
        description: "Add or subtract calendar days and find signed day differences.",
        syntax: "today + <whole number> days\ntoday - <whole number> days\nYYYY-MM-DD + <whole number> days\nYYYY-MM-DD - <whole number> days\ndays between YYYY-MM-DD and YYYY-MM-DD",
        examples: &[
            "today + 30 days",
            "2022-01-15 + 180 days",
            "days between 2022-01-15 and 2022-02-01",
        ],
        limitations: "today uses the local calendar date. Dates use ISO YYYY-MM-DD format. Separate the operator, count and days with spaces. Counts are nonnegative whole numbers. Differences are signed from the first date to the second. Time zones, clock times and business-day calendars are outside the supported grammar.",
    },
    HelpTopic {
        command: "currency",
        title: "Currency",
        description: "Convert fiat amounts using dated Frankfurter reference rates.",
        syntax: "<amount> <ISO code> to <ISO code>\n<amount> <ISO code> in <ISO code>\nUse three-letter currency codes such as USD, EUR and PHP. Codes are case-insensitive.",
        examples: &["25 usd to php", "100 eur to usd"],
        limitations: "Use decimal-point amounts within the supported Decimal range and 28 decimal places. The provider's fiat currency catalog determines availability. Reference rates have a displayed source date and omit live trading spreads and fees. Currency queries automatically fetch missing rates and refresh caches after 24 hours. Later currency queries can retry failed requests after a 60-second cooldown. Rates whose source date is at least three calendar days old are labeled stale. Cached rates remain usable offline. The first conversion needs a connection when no cached rate exists. Unavailable pairs and failed requests show an explanation. Matching source and destination codes need no rate fetch.",
    },
    HelpTopic {
        command: "search",
        title: "Applications and files",
        description: "Type an installed application name or an indexed file or folder name.",
        syntax: "<application name>\n<file or folder name>\nFull and partial names are matched case-insensitively, with exact, prefix, word, substring and fuzzy ranking.",
        examples: &["firefox", "Project notes"],
        limitations: "Application examples match when the app is installed. File examples match when the name exists in an indexed folder. Default file roots are Documents, Desktop and Downloads. Change roots or refresh search in Settings. Search uses names from a cached metadata index. Hidden, build and cache folders are skipped, directory symlinks are skipped, and file contents are outside the search scope. Open, Copy path and Reveal are explicit result actions.",
    },
];

#[derive(Default)]
pub struct HelpFeature;
impl HelpFeature {
    pub fn new() -> Self {
        Self
    }
}
impl Feature for HelpFeature {
    fn id(&self) -> &'static str {
        "help"
    }
    fn query(
        &self,
        request: &QueryRequest,
        _: &QueryContext,
        cancel: &Cancellation,
    ) -> FeatureReply {
        let mut words = request.text.split_whitespace();
        if cancel.is_cancelled()
            || !words
                .next()
                .is_some_and(|word| word.eq_ignore_ascii_case("help"))
        {
            return FeatureReply::NotApplicable;
        }
        let command = words.next();
        let topic = command.and_then(|command| {
            TOPICS
                .iter()
                .find(|topic| topic.command.eq_ignore_ascii_case(command))
        });
        if let Some(topic) = topic.filter(|_| words.next().is_none()) {
            return FeatureReply::ExclusiveResults(topic_results(topic));
        }
        let mut results = Vec::new();
        if command.is_some() {
            results.push(reference(
                "unknown",
                "Choose a help topic",
                "Use one of the help commands below.",
                None,
            ));
        }
        results.extend(TOPICS.iter().map(|topic| {
            reference(
                &format!("topic-{}", topic.command),
                topic.title,
                &format!("help {}\n{}", topic.command, topic.description),
                Some(QueryInput {
                    text: format!("help {}", topic.command),
                    submit: true,
                }),
            )
        }));
        FeatureReply::ExclusiveResults(results)
    }
}

fn topic_results(topic: &HelpTopic) -> Vec<ResultItem> {
    let mut results = vec![
        reference(
            &format!("{}-description", topic.command),
            topic.title,
            topic.description,
            None,
        ),
        reference(
            &format!("{}-syntax", topic.command),
            "Syntax",
            topic.syntax,
            None,
        ),
    ];
    results.extend(topic.examples.iter().enumerate().map(|(index, example)| {
        reference(
            &format!("{}-example-{index}", topic.command),
            example,
            "Edit this example, then press Enter to evaluate it.",
            Some(QueryInput {
                text: example.to_string(),
                submit: false,
            }),
        )
    }));
    results.push(reference(
        &format!("{}-limitations", topic.command),
        "Limitations",
        topic.limitations,
        None,
    ));
    results
}

fn reference(key: &str, title: &str, detail: &str, input: Option<QueryInput>) -> ResultItem {
    ResultItem {
        key: ResultKey {
            feature: "help",
            local: key.into(),
        },
        title: title.into(),
        detail: detail.into(),
        presentation: Presentation::Reference,
        actions: input
            .into_iter()
            .map(|input| Arc::new(SetQueryAction(input)) as Arc<dyn FeatureAction>)
            .collect(),
    }
}

struct SetQueryAction(QueryInput);
impl FeatureAction for SetQueryAction {
    fn label(&self) -> &str {
        if self.0.submit {
            "Open reference"
        } else {
            "Use example"
        }
    }
    fn execute(&self, _: &dyn ActionHost) -> Result<ActionOutcome, ActionError> {
        Ok(ActionOutcome {
            yield_focus: false,
            input: Some(self.0.clone()),
        })
    }
}
