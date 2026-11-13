use crate::{cancel::Cancellation, feature::*, number::format_number, query::*};

#[derive(Default)]
pub struct CalculatorFeature;
impl CalculatorFeature {
    pub fn new() -> Self {
        Self
    }
}
impl Feature for CalculatorFeature {
    fn id(&self) -> &'static str {
        "calculator"
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
        let q = request
            .text
            .trim()
            .replace('×', "*")
            .replace('÷', "/")
            .replace('−', "-")
            .replace('π', "pi");
        if !is_expression(&q) {
            return FeatureReply::NotApplicable;
        }
        if q.len() > MAX_QUERY_BYTES {
            return invalid("Expression is too long");
        }
        let mut depth: i32 = 0;
        for ch in q.chars() {
            if ch == '(' {
                depth += 1;
            }
            if ch == ')' {
                depth -= 1;
            }
            if depth > 64 {
                return invalid("Use at most 64 nested groups");
            }
            if depth < 0 {
                return invalid("Check the parentheses");
            }
        }
        if depth > 0 || q.ends_with(['+', '-', '*', '/', '^', '%', ',']) {
            return FeatureReply::Incomplete;
        }
        let factor = if context.angle_mode == AngleMode::Degrees {
            std::f64::consts::PI / 180.0
        } else {
            1.0
        };
        let mut ctx = meval::Context::new();
        ctx.func("log", f64::log10)
            .func("sin", move |x| (x * factor).sin())
            .func("cos", move |x| (x * factor).cos())
            .func("tan", move |x| (x * factor).tan())
            .func("asin", move |x| x.asin() / factor)
            .func("acos", move |x| x.acos() / factor)
            .func("atan", move |x| x.atan() / factor)
            .func2("atan2", move |a, b| a.atan2(b) / factor);
        match meval::eval_str_with_context(&q, &ctx) {
            Ok(value) => match format_number(value) {
                Ok(value) => {
                    let detail = if ["sin", "cos", "tan"].iter().any(|f| q.contains(f)) {
                        format!(
                            "Calculator · {}",
                            if context.angle_mode == AngleMode::Degrees {
                                "Degrees"
                            } else {
                                "Radians"
                            }
                        )
                    } else {
                        "Calculator · on your device".into()
                    };
                    FeatureReply::Results(vec![ResultItem::answer(
                        self.id(),
                        q,
                        value.clone(),
                        detail,
                        value,
                    )])
                }
                Err(_) => {
                    invalid("No finite result — check division by zero or the function's domain")
                }
            },
            Err(error) => invalid(&format!("Check the expression: {error}")),
        }
    }
}
fn invalid(message: &str) -> FeatureReply {
    FeatureReply::Invalid {
        message: message.into(),
    }
}
fn is_expression(q: &str) -> bool {
    if q.is_empty()
        || q.contains(" in ")
        || q.contains(" to ")
        || q.contains("days")
        || q.contains("today")
    {
        return false;
    }
    if q.len() >= 10 {
        let b = q.as_bytes();
        if b[4] == b'-' && b[7] == b'-' && b[..4].iter().all(u8::is_ascii_digit) {
            return false;
        }
    }
    let known = [
        "sqrt", "abs", "exp", "ln", "log", "sin", "cos", "tan", "asin", "acos", "atan", "atan2",
        "sinh", "cosh", "tanh", "asinh", "acosh", "atanh", "floor", "ceil", "round", "signum",
        "min", "max", "pi", "e",
    ];
    let mut ident = String::new();
    let mut has_function = false;
    let mut has_operator = false;
    for ch in q.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_alphabetic() || (!ident.is_empty() && ch.is_ascii_digit()) || ch == '_' {
            ident.push(ch);
        } else {
            if !ident.is_empty() {
                if !known.contains(&ident.as_str()) {
                    return false;
                }
                has_function = true;
                ident.clear();
            }
            if "+-*/^%()".contains(ch) {
                has_operator = true;
            } else if !ch.is_ascii_digit() && !ch.is_whitespace() && ch != '.' && ch != ',' {
                return false;
            }
        }
    }
    has_function || has_operator
}
