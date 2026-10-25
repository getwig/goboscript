// Adapted from: https://github.com/boa-dev/boa
// MIT License
//
// Copyright (c) 2019 Jason Williams
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

/// Helper function to check if a `char` is trimmable.
const fn is_trimmable_whitespace(c: char) -> bool {
    // The rust implementation of `trim` does not regard the same characters whitespace as ecma standard does
    //
    // Rust uses \p{White_Space} by default, which also includes:
    // `\u{0085}' (next line)
    // And does not include:
    // '\u{FEFF}' (zero width non-breaking space)
    // Explicit whitespace: https://tc39.es/ecma262/#sec-white-space
    matches!(
        c,
        '\u{0009}' | '\u{000B}' | '\u{000C}' | '\u{0020}' | '\u{00A0}' | '\u{FEFF}' |
    // Unicode Space_Separator category
    '\u{1680}' | '\u{2000}'
            ..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}' |
    // Line terminators: https://tc39.es/ecma262/#sec-line-terminators
    '\u{000A}' | '\u{000D}' | '\u{2028}' | '\u{2029}'
    )
}

pub fn parse_float(input_string: &str) -> f64 {
    let s = input_string.trim_start_matches(is_trimmable_whitespace);
    let s_prefix = s.chars().take(4).collect::<String>();
    let s_prefix_lower = s_prefix.to_ascii_lowercase();
    // TODO: write our own lexer to match syntax StrDecimalLiteral
    if s.starts_with("Infinity") || s.starts_with("+Infinity") {
        f64::INFINITY
    } else if s.starts_with("-Infinity") {
        f64::NEG_INFINITY
    } else if s_prefix_lower.starts_with("inf")
        || s_prefix_lower.starts_with("+inf")
        || s_prefix_lower.starts_with("-inf")
    {
        // Prevent fast_float from parsing "inf", "+inf" as Infinity and "-inf" as -Infinity
        f64::NAN
    } else {
        fast_float2::parse_partial::<f64, _>(s).map_or_else(
            |_| f64::NAN,
            |(f, len)| {
                if len > 0 {
                    f
                } else {
                    f64::NAN
                }
            },
        )
    }
}

/// `Number.prototype.toString()` (radix 10) as specified by ECMAScript
/// (Number::toString). Scratch converts numbers to text this way, e.g. in `join`.
pub fn number_to_string(number: f64) -> String {
    if number.is_nan() {
        return "NaN".into();
    }
    if number == 0.0 {
        return "0".into();
    }
    if number.is_infinite() {
        return if number > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if number < 0.0 {
        return format!("-{}", number_to_string(-number));
    }
    // Rust's `{:e}` prints the shortest digits that round-trip, like ECMAScript requires.
    let exp_form = format!("{:e}", number);
    let (mantissa, exponent) = exp_form.split_once('e').unwrap();
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exponent.parse::<i32>().unwrap() + 1;
    if k <= n && n <= 21 {
        format!("{}{}", digits, "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{}", "0".repeat((-n) as usize), digits)
    } else {
        let sign = if n - 1 < 0 { "-" } else { "+" };
        let e = (n - 1).abs();
        if k == 1 {
            format!("{}e{}{}", digits, sign, e)
        } else {
            format!("{}.{}e{}{}", &digits[..1], &digits[1..], sign, e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::number_to_string;

    #[test]
    fn number_to_string_matches_ecmascript() {
        let cases: &[(f64, &str)] = &[
            (0.0, "0"),
            (-0.0, "0"),
            (1.0, "1"),
            (-7.0, "-7"),
            (0.5, "0.5"),
            (0.1 + 0.2, "0.30000000000000004"),
            (123456.78901234567, "123456.78901234567"),
            (111.00000000000001, "111.00000000000001"),
            (1e20, "100000000000000000000"),
            (1e21, "1e+21"),
            (1.5e21, "1.5e+21"),
            (1e300, "1e+300"),
            (9223372036854775807.0, "9223372036854776000"),
            (1e-6, "0.000001"),
            (1e-7, "1e-7"),
            (1.25e-7, "1.25e-7"),
            (f64::NAN, "NaN"),
            (f64::INFINITY, "Infinity"),
            (f64::NEG_INFINITY, "-Infinity"),
            (9007199254740993.0, "9007199254740992"),
        ];
        for (n, s) in cases {
            assert_eq!(number_to_string(*n), *s, "for {n:?}");
        }
    }
}
