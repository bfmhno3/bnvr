use std::error::Error;
use std::time::Duration;

pub fn parse_duration(input: &str) -> Result<Duration, Box<dyn Error>> {
    let suffix = input
        .chars()
        .rev()
        .take_while(|c| c.is_alphabetic())
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    if suffix.is_empty() {
        return Err(format!("invalid duration: {input}").into());
    }

    let number = input[..input.len() - suffix.len()].trim();
    if number.starts_with('-') {
        return Err(format!("invalid duration: {input}").into());
    }
    let value = number.parse::<u64>()?;
    if value == 0 {
        return Err(format!("invalid duration: {input}").into());
    }

    let seconds = match suffix.as_str() {
        "y" => value.saturating_mul(365 * 24 * 60 * 60),
        "d" => value.saturating_mul(24 * 60 * 60),
        "m" => value.saturating_mul(60),
        "s" => value,
        _ => return Err(format!("invalid duration suffix: {suffix}").into()),
    };
    Ok(Duration::from_secs(seconds))
}

pub fn validate_auto_sync_timeout(
    auto_sync: Option<&str>,
    timeout: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let auto_sync_duration = auto_sync.map(parse_duration).transpose()?;
    let timeout_duration = timeout.map(parse_duration).transpose()?;

    if let (Some(auto_sync_duration), Some(timeout_duration)) =
        (auto_sync_duration, timeout_duration)
        && auto_sync_duration < timeout_duration
    {
        eprintln!(
            "warning: auto-sync interval is less than timeout; using timeout as auto-sync interval"
        );
    }

    Ok(())
}

pub fn effective_auto_sync_duration(
    auto_sync: &str,
    timeout: Option<&str>,
) -> Result<Duration, Box<dyn Error>> {
    let auto_sync_duration = parse_duration(auto_sync)?;
    let Some(timeout) = timeout else {
        return Ok(auto_sync_duration);
    };
    let timeout_duration = parse_duration(timeout)?;
    Ok(auto_sync_duration.max(timeout_duration))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_units() {
        assert_eq!(parse_duration("1s").unwrap(), Duration::from_secs(1));
        assert_eq!(parse_duration("2m").unwrap(), Duration::from_secs(120));
        assert_eq!(parse_duration("3d").unwrap(), Duration::from_secs(259200));
        assert_eq!(parse_duration("1y").unwrap(), Duration::from_secs(31536000));
    }

    #[test]
    fn test_parse_duration_rejects_invalid_input() {
        for value in ["", "10", "0s", "-1s", "1h"] {
            assert!(parse_duration(value).is_err(), "{value} should fail");
        }
    }

    #[test]
    fn test_effective_auto_sync_clamps_to_timeout() {
        assert_eq!(
            effective_auto_sync_duration("10s", Some("30s")).unwrap(),
            Duration::from_secs(30)
        );
    }
}
