#[derive(Debug)]
pub struct NumberError;
impl std::fmt::Display for NumberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("The result is outside the supported number range.")
    }
}
impl std::error::Error for NumberError {}

pub fn format_number(value: f64) -> Result<String, NumberError> {
    if !value.is_finite() {
        return Err(NumberError);
    }
    if value == 0.0 {
        return Ok("0".into());
    }
    let magnitude = value.abs();
    if !(1e-6..1e12).contains(&magnitude) {
        let raw = format!("{value:.11e}");
        let (mantissa, exponent) = raw.split_once('e').ok_or(NumberError)?;
        let exponent: i32 = exponent.parse().map_err(|_| NumberError)?;
        return Ok(format!("{}e{exponent}", trim(mantissa)));
    }
    let places = (11.0 - magnitude.log10().floor()).clamp(0.0, 16.0) as usize;
    Ok(trim(&format!("{value:.places$}")))
}
fn trim(text: &str) -> String {
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').into()
    } else {
        text.into()
    }
}
