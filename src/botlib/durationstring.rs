#[derive(PartialEq, Debug)]
pub enum Unit {
    Seconds,
    Minutes,
    Hours,
    Days,
    Weeks,
}

impl Unit {
    /// Convert the unit to seconds
    pub fn to_seconds(&self) -> u64 {
        match self {
            Unit::Seconds => 1,
            Unit::Minutes => 60,
            Unit::Hours => 3600,
            Unit::Days => 86400,
            Unit::Weeks => 604800,
        }
    }
}

impl TryFrom<&str> for Unit {
    type Error = crate::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "seconds" => Ok(Unit::Seconds),
            "second" => Ok(Unit::Seconds), // Allow "second" as a shorthand for "seconds"
            "secs" => Ok(Unit::Seconds),   // Allow "secs" as a shorthand for "seconds"
            "sec" => Ok(Unit::Seconds),    // Allow "sec" as a shorthand for "seconds"
            "s" => Ok(Unit::Seconds),      // Allow "s" as a shorthand for "seconds"
            "minutes" => Ok(Unit::Minutes),
            "minute" => Ok(Unit::Minutes), // Allow "minute" as a shorthand for "minutes"
            "mins" => Ok(Unit::Minutes),   // Allow "mins" as a shorthand for "minutes"
            "min" => Ok(Unit::Minutes),    // Allow "min" as a shorthand for "minutes"
            "m" => Ok(Unit::Minutes),      // Allow "m" as a shorthand for "minutes"
            "hours" => Ok(Unit::Hours),
            "hour" => Ok(Unit::Hours), // Allow "hour" as a shorthand for "hours"
            "hrs" => Ok(Unit::Hours),  // Allow "hrs" as a shorthand for "hours"
            "hr" => Ok(Unit::Hours),   // Allow "hr" as a shorthand for "hours"
            "h" => Ok(Unit::Hours),    // Allow "h" as a shorthand for "hours"
            "days" => Ok(Unit::Days),
            "day" => Ok(Unit::Days), // Allow "day" as a shorthand for "days"
            "d" => Ok(Unit::Days),   // Allow "d" as a shorthand for "days"
            "weeks" => Ok(Unit::Weeks),
            "week" => Ok(Unit::Weeks), // Allow "week" as a shorthand for "weeks"
            "w" => Ok(Unit::Weeks),    // Allow "w" as a shorthand for "weeks"
            _ => Err("Invalid unit".into()),
        }
    }
}

/// Given a string of the format <number> days/hours/minutes/seconds, parse it into a u64 of seconds
///
/// This function should handle both spaced and non-spaced formats
pub fn parse_duration_string(s: &str) -> Result<(u64, Unit), crate::Error> {
    let mut number: u64 = 0;
    let mut unit = String::new();

    // Keep looping adding up each number until we hit a non-number which gets added to unit
    for c in s.chars() {
        if c.is_numeric() {
            number = number * 10 + c.to_digit(10).ok_or("Cannot convert to integer")? as u64;
        } else {
            if c == ' ' {
                continue;
            }

            unit.push(c);
        }
    }

    let unit = Unit::try_from(unit.as_str())?;

    Ok((number, unit))
}
