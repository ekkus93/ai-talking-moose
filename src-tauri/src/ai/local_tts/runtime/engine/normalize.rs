use once_cell::sync::Lazy;
use regex::{Captures, Regex};

use super::super::LocalTtsRuntimeError;

pub(super) const MAX_INPUT_CHARS: usize = 500;

static DATE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(January|February|March|April|May|June|July|August|September|October|November|December)\s+(\d{1,2})(?:st|nd|rd|th)?,\s+(\d{4})\b")
        .expect("static date regex")
});
static CURRENCY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\$(-?\d+(?:\.\d+)?)([KMB])?\b").expect("static currency regex"));
static UNIT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(\d+(?:\.\d+)?)\s*(kHz|MHz|GHz|Hz|KB|MB|GB)\b").expect("static unit regex")
});
static PERCENT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(-?\d+(?:\.\d+)?)%").expect("static percent regex"));
static TIME_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(\d{1,2}):(\d{2})(?:\s*(AM|PM|am|pm))?\b").expect("static time regex")
});
static VERSION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b([A-Za-z][A-Za-z0-9_]*)-(\d+(?:\.\d+)+)\b").expect("static version regex")
});
static ORDINAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(\d+)(st|nd|rd|th)\b").expect("static ordinal regex"));
static NUMBER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(^|[^A-Za-z0-9_])(-?\d+(?:\.\d+)?)([^A-Za-z0-9_]|$)").expect("static number regex")
});

pub(super) fn normalize_text(text: &str) -> Result<String, LocalTtsRuntimeError> {
    if text
        .chars()
        .any(|character| character.is_control() && !character.is_whitespace())
    {
        return Err(LocalTtsRuntimeError::invalid_input());
    }

    let mut normalized = text.replace(['"', '\u{201c}', '\u{201d}'], "");
    for (pattern, replacement) in [
        ("Dr.", "Doctor"),
        ("Fig.", "Figure"),
        ("Mr.", "Mister"),
        ("Mrs.", "Missus"),
        ("Ms.", "Miss"),
    ] {
        normalized = normalized.replace(pattern, replacement);
    }

    normalized = DATE_RE
        .replace_all(&normalized, |captures: &Captures<'_>| {
            let day = captures[2].parse::<u64>().unwrap_or(0);
            let year = captures[3].parse::<u64>().unwrap_or(0);
            format!(
                "{} {}, {}",
                &captures[1],
                ordinal_to_words(day),
                integer_to_words(year)
            )
        })
        .into_owned();

    normalized = CURRENCY_RE
        .replace_all(&normalized, |captures: &Captures<'_>| {
            currency_to_words(&captures[1], captures.get(2).map(|value| value.as_str()))
        })
        .into_owned();

    normalized = UNIT_RE
        .replace_all(&normalized, |captures: &Captures<'_>| {
            let amount = decimal_to_words(&captures[1]);
            let singular = numeric_value_is_one(&captures[1]);
            let unit = match captures[2].to_ascii_lowercase().as_str() {
                "khz" => "kilohertz",
                "mhz" => "megahertz",
                "ghz" => "gigahertz",
                "hz" => "hertz",
                "kb" if singular => "kilobyte",
                "kb" => "kilobytes",
                "mb" if singular => "megabyte",
                "mb" => "megabytes",
                "gb" if singular => "gigabyte",
                "gb" => "gigabytes",
                _ => "",
            };
            format!("{amount} {unit}")
        })
        .into_owned();

    normalized = PERCENT_RE
        .replace_all(&normalized, |captures: &Captures<'_>| {
            format!("{} percent", signed_decimal_to_words(&captures[1]))
        })
        .into_owned();

    normalized = TIME_RE
        .replace_all(&normalized, |captures: &Captures<'_>| {
            let hour = captures[1].parse::<u64>().unwrap_or(0);
            let minute = captures[2].parse::<u64>().unwrap_or(0);
            let minute_words = if minute < 10 {
                format!("oh {}", integer_to_words(minute))
            } else {
                integer_to_words(minute)
            };
            match captures.get(3) {
                Some(period) => format!(
                    "{} {} {}",
                    integer_to_words(hour),
                    minute_words,
                    period.as_str().to_ascii_uppercase()
                ),
                None => format!("{} {}", integer_to_words(hour), minute_words),
            }
        })
        .into_owned();

    normalized = VERSION_RE
        .replace_all(&normalized, |captures: &Captures<'_>| {
            format!("{} {}", &captures[1], decimal_to_words(&captures[2]))
        })
        .into_owned();

    normalized = ORDINAL_RE
        .replace_all(&normalized, |captures: &Captures<'_>| {
            captures[1]
                .parse::<u64>()
                .ok()
                .map(ordinal_to_words)
                .unwrap_or_else(|| captures[0].to_string())
        })
        .into_owned();

    normalized = NUMBER_RE
        .replace_all(&normalized, |captures: &Captures<'_>| {
            format!(
                "{}{}{}",
                &captures[1],
                signed_decimal_to_words(&captures[2]),
                &captures[3]
            )
        })
        .into_owned();

    normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    let length = normalized.chars().count();
    if length == 0 || length > MAX_INPUT_CHARS {
        return Err(LocalTtsRuntimeError::invalid_input());
    }
    Ok(normalized)
}

fn numeric_value_is_one(value: &str) -> bool {
    value.parse::<f64>().is_ok_and(|value| value == 1.0)
}

fn currency_to_words(value: &str, scale: Option<&str>) -> String {
    let negative = value.starts_with('-');
    let unsigned = value.trim_start_matches('-');
    let sign = if negative { "negative " } else { "" };

    if let Some(scale) = scale {
        let scale_word = match scale {
            "K" => "thousand",
            "M" => "million",
            "B" => "billion",
            _ => "",
        };
        return format!("{sign}{} {scale_word} dollars", decimal_to_words(unsigned));
    }

    let mut parts = unsigned.splitn(2, '.');
    let dollars = parts.next().unwrap_or("0").parse::<u64>().unwrap_or(0);
    let cents = parts.next().map(|fraction| {
        let mut digits = fraction.chars().take(2).collect::<String>();
        while digits.len() < 2 {
            digits.push('0');
        }
        digits.parse::<u64>().unwrap_or(0)
    });
    let dollar_unit = if dollars == 1 { "dollar" } else { "dollars" };
    let dollar_words = format!("{sign}{} {dollar_unit}", integer_to_words(dollars));
    match cents {
        Some(cents) if cents > 0 => {
            let cent_unit = if cents == 1 { "cent" } else { "cents" };
            format!("{dollar_words} and {} {cent_unit}", integer_to_words(cents))
        }
        _ => dollar_words,
    }
}

fn signed_decimal_to_words(value: &str) -> String {
    match value.strip_prefix('-') {
        Some(unsigned) => format!("negative {}", decimal_to_words(unsigned)),
        None => decimal_to_words(value),
    }
}

fn decimal_to_words(value: &str) -> String {
    let mut parts = value.splitn(2, '.');
    let whole = parts.next().unwrap_or("0").parse::<u64>().unwrap_or(0);
    match parts.next() {
        Some(fraction) if !fraction.is_empty() => {
            let digits = fraction
                .chars()
                .filter_map(|digit| digit.to_digit(10))
                .map(|digit| digit_word(digit as u8))
                .collect::<Vec<_>>()
                .join(" ");
            format!("{} point {digits}", integer_to_words(whole))
        }
        _ => integer_to_words(whole),
    }
}

fn integer_to_words(value: u64) -> String {
    match value {
        0..=19 => SMALL[value as usize].to_string(),
        20..=99 => {
            let tens = value / 10;
            let remainder = value % 10;
            if remainder == 0 {
                TENS[tens as usize].to_string()
            } else {
                format!("{}-{}", TENS[tens as usize], SMALL[remainder as usize])
            }
        }
        100..=999 => {
            let hundreds = value / 100;
            let remainder = value % 100;
            if remainder == 0 {
                format!("{} hundred", SMALL[hundreds as usize])
            } else {
                format!(
                    "{} hundred {}",
                    SMALL[hundreds as usize],
                    integer_to_words(remainder)
                )
            }
        }
        1_000..=999_999 => scale_words(value, 1_000, "thousand"),
        1_000_000..=999_999_999 => scale_words(value, 1_000_000, "million"),
        1_000_000_000..=999_999_999_999 => scale_words(value, 1_000_000_000, "billion"),
        _ => value.to_string(),
    }
}

fn scale_words(value: u64, scale: u64, label: &str) -> String {
    let major = value / scale;
    let remainder = value % scale;
    if remainder == 0 {
        format!("{} {label}", integer_to_words(major))
    } else {
        format!(
            "{} {label} {}",
            integer_to_words(major),
            integer_to_words(remainder)
        )
    }
}

fn ordinal_to_words(value: u64) -> String {
    match value {
        0 => "zeroth".to_string(),
        1 => "first".to_string(),
        2 => "second".to_string(),
        3 => "third".to_string(),
        4 => "fourth".to_string(),
        5 => "fifth".to_string(),
        6 => "sixth".to_string(),
        7 => "seventh".to_string(),
        8 => "eighth".to_string(),
        9 => "ninth".to_string(),
        10 => "tenth".to_string(),
        11 => "eleventh".to_string(),
        12 => "twelfth".to_string(),
        13 => "thirteenth".to_string(),
        14 => "fourteenth".to_string(),
        15 => "fifteenth".to_string(),
        16 => "sixteenth".to_string(),
        17 => "seventeenth".to_string(),
        18 => "eighteenth".to_string(),
        19 => "nineteenth".to_string(),
        20 => "twentieth".to_string(),
        21..=99 => {
            let tens = value / 10;
            let remainder = value % 10;
            if remainder == 0 {
                format!("{}ieth", TENS[tens as usize].trim_end_matches('y'))
            } else {
                format!("{}-{}", TENS[tens as usize], ordinal_to_words(remainder))
            }
        }
        _ => {
            let cardinal = integer_to_words(value);
            format!("{cardinal}th")
        }
    }
}

fn digit_word(value: u8) -> &'static str {
    SMALL[value as usize]
}

const SMALL: [&str; 20] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];

const TENS: [&str; 10] = [
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];

#[cfg(test)]
mod tests {
    use super::super::super::LocalTtsRuntimeErrorKind;
    use super::*;

    #[test]
    fn frozen_p0_semantic_normalization_targets_are_preserved() {
        let corpus: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../tests/fixtures/kittentts_p0_reference_corpus.json"
        ))
        .unwrap();
        for case in corpus["cases"].as_array().unwrap() {
            let text = case["text"].as_str().unwrap();
            let expected = case["normalized_text"].as_str().unwrap();
            assert_eq!(
                normalize_text(text).unwrap(),
                expected,
                "case {}",
                case["id"]
            );
        }
    }

    #[test]
    fn input_normalization_is_bounded_and_rejects_empty_or_control_text() {
        assert_eq!(normalize_text("  hello\n\tworld  ").unwrap(), "hello world");
        assert_eq!(
            normalize_text("   ").unwrap_err().kind,
            LocalTtsRuntimeErrorKind::InvalidInput
        );
        assert_eq!(
            normalize_text("hello\0world").unwrap_err().kind,
            LocalTtsRuntimeErrorKind::InvalidInput
        );
        assert_eq!(
            normalize_text(&"x".repeat(MAX_INPUT_CHARS + 1))
                .unwrap_err()
                .kind,
            LocalTtsRuntimeErrorKind::InvalidInput
        );
    }
}
