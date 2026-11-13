use crate::{cancel::Cancellation, feature::*, number::format_number, query::*};

#[derive(Clone, Copy, PartialEq)]
enum Dimension {
    Length,
    Mass,
    Temperature,
    Area,
    Volume,
    Speed,
    Duration,
    Storage,
}
#[derive(Clone, Copy)]
struct Unit {
    dimension: Dimension,
    scale: f64,
    offset: f64,
    symbol: &'static str,
}
impl Unit {
    const fn new(dimension: Dimension, scale: f64, symbol: &'static str) -> Self {
        Self {
            dimension,
            scale,
            offset: 0.0,
            symbol,
        }
    }
}
#[derive(Default)]
pub struct MeasurementFeature;
impl MeasurementFeature {
    pub fn new() -> Self {
        Self
    }
}
impl Feature for MeasurementFeature {
    fn id(&self) -> &'static str {
        "measurements"
    }
    fn query(
        &self,
        request: &QueryRequest,
        _: &QueryContext,
        cancel: &Cancellation,
    ) -> FeatureReply {
        if cancel.is_cancelled() {
            return FeatureReply::NotApplicable;
        }
        let q = request.text.trim();
        let lower = q.to_ascii_lowercase();
        let Some((at, sep)) = [" in ", " to "]
            .iter()
            .filter_map(|sep| lower.rfind(sep).map(|at| (at, sep.len())))
            .max()
        else {
            return FeatureReply::NotApplicable;
        };
        let (lhs, rhs) = (q[..at].trim(), q[at + sep..].trim());
        let number_end = lhs
            .char_indices()
            .find(|(_, c)| !c.is_ascii_digit() && !".,+-eE".contains(*c))
            .map(|(i, _)| i)
            .unwrap_or(lhs.len());
        let from = resolve(lhs[number_end..].trim());
        let to = resolve(rhs);
        if from.is_empty() || to.is_empty() {
            return FeatureReply::NotApplicable;
        }
        let numeric = &lhs[..number_end];
        if numeric.contains(',') {
            return invalid("Use a decimal point; comma-separated measurements are ambiguous");
        }
        let Ok(amount) = numeric.parse::<f64>() else {
            return FeatureReply::Incomplete;
        };
        if !amount.is_finite() {
            return invalid("Use a finite measurement");
        }
        if amount == 0.0
            && numeric
                .split(['e', 'E'])
                .next()
                .is_some_and(|mantissa| mantissa.bytes().any(|b| matches!(b, b'1'..=b'9')))
        {
            return invalid("The measurement is smaller than the supported number range");
        }
        if from[0].dimension != to[0].dimension {
            return invalid("These units measure different things");
        }
        let ambiguous = from.len() > 1 || to.len() > 1;
        let mut results = Vec::new();
        if ambiguous {
            results.push(ResultItem::explanation(
                self.id(),
                "Choose a gallon definition",
                "US and imperial gallons have different volumes. Select a defined result below.",
            ));
        }
        for a in from {
            for b in &to {
                let canonical = amount * a.scale + a.offset;
                if a.dimension == Dimension::Temperature && canonical < -273.15 - 1e-10 {
                    return invalid("Temperature is below absolute zero");
                }
                let value = (canonical - b.offset) / b.scale;
                if amount != 0.0 && value == 0.0 && a.offset == 0.0 && b.offset == 0.0 {
                    return invalid("Conversion is smaller than the supported number range");
                }
                let Ok(number) = format_number(value) else {
                    return invalid("Conversion is outside the supported number range");
                };
                let label = if ambiguous {
                    format!(
                        " ({})",
                        if a.symbol.contains("gal") {
                            a.symbol
                        } else {
                            b.symbol
                        }
                    )
                } else {
                    String::new()
                };
                let copy = format!("{number} {}", b.symbol);
                results.push(ResultItem::answer(
                    self.id(),
                    format!("{}-{}", a.symbol, b.symbol),
                    format!("{copy}{label}"),
                    format!("{amount} {} → {}", a.symbol, b.symbol),
                    copy,
                ));
            }
        }
        FeatureReply::Results(results)
    }
}
fn invalid(message: &str) -> FeatureReply {
    FeatureReply::Invalid {
        message: message.into(),
    }
}
fn resolve(raw: &str) -> Vec<Unit> {
    use Dimension::*;
    let storage = match raw {
        "kb" | "Kb" => Some((125.0, "kb")),
        "Mb" => Some((125000.0, "Mb")),
        "Gb" => Some((125000000.0, "Gb")),
        "Tb" => Some((125000000000.0, "Tb")),
        "Kib" => Some((128.0, "Kib")),
        "Mib" => Some((131072.0, "Mib")),
        "Gib" => Some((134217728.0, "Gib")),
        "Tib" => Some((137438953472.0, "Tib")),
        "kB" | "KB" => Some((1000.0, "kB")),
        "MB" => Some((1e6, "MB")),
        "GB" => Some((1e9, "GB")),
        "TB" => Some((1e12, "TB")),
        "KiB" => Some((1024.0, "KiB")),
        "MiB" => Some((1048576.0, "MiB")),
        "GiB" => Some((1073741824.0, "GiB")),
        "TiB" => Some((1099511627776.0, "TiB")),
        _ => None,
    };
    if let Some((scale, symbol)) = storage {
        return vec![Unit::new(Storage, scale, symbol)];
    }
    // Short storage symbols are case-sensitive. Long unit names remain case-insensitive.
    if raw.len() <= 3
        && ["kb", "mb", "gb", "tb", "kib", "mib", "gib", "tib"]
            .contains(&raw.to_ascii_lowercase().as_str())
    {
        return vec![];
    }
    // The single-character bit/byte symbols are case-sensitive.
    if raw == "B" {
        return vec![Unit::new(Storage, 1.0, "B")];
    }
    if raw == "b" {
        return vec![Unit::new(Storage, 0.125, "bit")];
    }
    let key = raw
        .to_ascii_lowercase()
        .replace('²', "2")
        .replace('³', "3")
        .replace(['µ', 'μ'], "u");
    let unit = match key.as_str() {
        "m" | "meter" | "meters" | "metre" | "metres" => Unit::new(Length, 1.0, "m"),
        "mm" | "millimeter" | "millimeters" => Unit::new(Length, 0.001, "mm"),
        "um" | "micrometer" | "micrometers" => Unit::new(Length, 0.000001, "µm"),
        "cm" | "centimeter" | "centimeters" => Unit::new(Length, 0.01, "cm"),
        "km" | "kilometer" | "kilometers" => Unit::new(Length, 1000.0, "km"),
        "in" | "inch" | "inches" => Unit::new(Length, 0.0254, "in"),
        "ft" | "foot" | "feet" => Unit::new(Length, 0.3048, "ft"),
        "yd" | "yard" | "yards" => Unit::new(Length, 0.9144, "yd"),
        "mi" | "mile" | "miles" => Unit::new(Length, 1609.344, "mi"),
        "nmi" | "nautical mile" | "nautical miles" => Unit::new(Length, 1852.0, "nmi"),
        "kg" | "kilogram" | "kilograms" => Unit::new(Mass, 1.0, "kg"),
        "g" | "gram" | "grams" => Unit::new(Mass, 0.001, "g"),
        "mg" | "milligram" | "milligrams" => Unit::new(Mass, 0.000001, "mg"),
        "t" | "tonne" | "tonnes" => Unit::new(Mass, 1000.0, "t"),
        "lb" | "lbs" | "pound" | "pounds" => Unit::new(Mass, 0.45359237, "lb"),
        "oz" | "ounce" | "ounces" => Unit::new(Mass, 0.028349523125, "oz"),
        "st" | "stone" | "stones" => Unit::new(Mass, 6.35029318, "st"),
        "c" | "°c" | "celsius" => Unit::new(Temperature, 1.0, "°C"),
        "f" | "°f" | "fahrenheit" => Unit {
            dimension: Temperature,
            scale: 5.0 / 9.0,
            offset: -32.0 * 5.0 / 9.0,
            symbol: "°F",
        },
        "k" | "kelvin" => Unit {
            dimension: Temperature,
            scale: 1.0,
            offset: -273.15,
            symbol: "K",
        },
        "m2" | "square meter" | "square meters" => Unit::new(Area, 1.0, "m²"),
        "cm2" => Unit::new(Area, 0.0001, "cm²"),
        "km2" => Unit::new(Area, 1e6, "km²"),
        "ft2" | "sq ft" => Unit::new(Area, 0.09290304, "ft²"),
        "ha" | "hectare" | "hectares" => Unit::new(Area, 10000.0, "ha"),
        "ac" | "acre" | "acres" => Unit::new(Area, 4046.8564224, "ac"),
        "m3" => Unit::new(Volume, 1000.0, "m³"),
        "l" | "liter" | "liters" | "litre" | "litres" => Unit::new(Volume, 1.0, "L"),
        "ml" | "milliliter" | "milliliters" => Unit::new(Volume, 0.001, "mL"),
        "usgal" | "us gal" | "us gallon" | "us gallons" => Unit::new(Volume, 3.785411784, "US gal"),
        "impgal" | "imp gal" | "imperial gallon" | "imperial gallons" => {
            Unit::new(Volume, 4.54609, "imp gal")
        }
        "gal" | "gallon" | "gallons" => {
            return vec![
                Unit::new(Volume, 3.785411784, "US gal"),
                Unit::new(Volume, 4.54609, "imp gal"),
            ];
        }
        "m/s" | "mps" => Unit::new(Speed, 1.0, "m/s"),
        "km/h" | "kph" | "kmh" => Unit::new(Speed, 1.0 / 3.6, "km/h"),
        "mph" | "mi/h" => Unit::new(Speed, 0.44704, "mph"),
        "knot" | "knots" | "kn" => Unit::new(Speed, 1852.0 / 3600.0, "kn"),
        "ft/s" | "fps" => Unit::new(Speed, 0.3048, "ft/s"),
        "s" | "sec" | "second" | "seconds" => Unit::new(Duration, 1.0, "s"),
        "ms" | "millisecond" | "milliseconds" => Unit::new(Duration, 0.001, "ms"),
        "min" | "minute" | "minutes" => Unit::new(Duration, 60.0, "min"),
        "h" | "hr" | "hour" | "hours" => Unit::new(Duration, 3600.0, "h"),
        "d" | "day" | "days" => Unit::new(Duration, 86400.0, "days"),
        "wk" | "week" | "weeks" => Unit::new(Duration, 604800.0, "weeks"),
        "bit" | "bits" => Unit::new(Storage, 0.125, "bit"),
        "byte" | "bytes" => Unit::new(Storage, 1.0, "B"),
        "kilobyte" | "kilobytes" => Unit::new(Storage, 1000.0, "kB"),
        "megabyte" | "megabytes" => Unit::new(Storage, 1e6, "MB"),
        "gigabyte" | "gigabytes" => Unit::new(Storage, 1e9, "GB"),
        "terabyte" | "terabytes" => Unit::new(Storage, 1e12, "TB"),
        "kibibyte" | "kibibytes" => Unit::new(Storage, 1024.0, "KiB"),
        "mebibyte" | "mebibytes" => Unit::new(Storage, 1048576.0, "MiB"),
        "gibibyte" | "gibibytes" => Unit::new(Storage, 1073741824.0, "GiB"),
        "tebibyte" | "tebibytes" => Unit::new(Storage, 1099511627776.0, "TiB"),
        _ => return Vec::new(),
    };
    vec![unit]
}
