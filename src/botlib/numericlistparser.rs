use std::sync::LazyLock;

pub static REPLACE_CHANNEL: LazyLock<Vec<(&'static str, &'static str)>> =
    LazyLock::new(|| vec![("<#", ""), (">", "")]);

/// Parse a numeric list from a string without knowing its separator
pub fn parse_numeric_list<T: std::str::FromStr + Send + Sync>(
    s: &str,
    replace: &[(&'static str, &'static str)],
) -> Result<Vec<T>, T::Err> {
    let mut list = Vec::new();
    let mut number = String::new();

    for c in s.chars() {
        if c.is_numeric() {
            number.push(c);
        } else if !number.is_empty() {
            for (from, to) in replace {
                number = number.replace(from, to);
            }
            list.push(number.parse::<T>()?);
            number.clear();
        }
    }

    if !number.is_empty() {
        list.push(number.parse::<T>()?);
    }

    Ok(list)
}

/// Parse a numeric list from a string without knowing its separator, returning a string instead of a number
#[allow(dead_code)]
pub fn parse_numeric_list_to_str<T: std::fmt::Display + std::str::FromStr + Send + Sync>(
    s: &str,
    replace: &[(&'static str, &'static str)],
) -> Result<Vec<String>, T::Err> {
    parse_numeric_list::<T>(s, replace).map(|v| v.into_iter().map(|v| v.to_string()).collect())
}
