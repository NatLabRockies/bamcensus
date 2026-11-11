use std::fmt;

/// Defines how LODES API download failures should be handled for a given state/year combination.
///
/// The LODES dataset does not have complete coverage across all states and years. This enum
/// allows users to specify fallback behavior when a download fails (e.g., state/year not available).
///
/// # Examples
///
/// ```ignore
/// use bamcensus_lehd::model::RetryPolicy;
///
/// // Always fail if a download is not available
/// let policy = RetryPolicy::FailAlways;
///
/// // Try the previous year if current year fails
/// let policy = RetryPolicy::TryPreviousYears(1);
///
/// // Try the next year if current year fails
/// let policy = RetryPolicy::TryNextYears(1);
///
/// // Try 2 years back or 1 year forward
/// let policy = RetryPolicy::BiDirectional { past: 2, future: 1 };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryPolicy {
    /// Do not retry; fail immediately if the requested state/year is unavailable
    FailAlways,
    /// If the requested year fails, try N years earlier
    TryPreviousYears(u64),
    /// If the requested year fails, try N years later
    TryNextYears(u64),
    /// If the requested year fails, try up to N years earlier or M years later
    BiDirectional { past: u64, future: u64 },
}

impl RetryPolicy {
    /// Returns all years to attempt for a given base year, in priority order
    pub fn candidate_years(&self, base_year: u64) -> Vec<u64> {
        match self {
            RetryPolicy::FailAlways => vec![base_year],
            RetryPolicy::TryPreviousYears(n) => {
                let mut years = vec![base_year];
                for i in 1..=*n {
                    if base_year >= i {
                        years.push(base_year - i);
                    }
                }
                years
            }
            RetryPolicy::TryNextYears(n) => {
                let mut years = vec![base_year];
                for i in 1..=*n {
                    years.push(base_year + i);
                }
                years
            }
            RetryPolicy::BiDirectional { past, future } => {
                let mut years = vec![base_year];
                // Try past years in reverse chronological order
                for i in 1..=*past {
                    if base_year >= i {
                        years.push(base_year - i);
                    }
                }
                // Try future years in chronological order
                for i in 1..=*future {
                    years.push(base_year + i);
                }
                years
            }
        }
    }
}

impl fmt::Display for RetryPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RetryPolicy::FailAlways => write!(f, "fail_always"),
            RetryPolicy::TryPreviousYears(n) => write!(f, "try_{}_years_back", n),
            RetryPolicy::TryNextYears(n) => write!(f, "try_{}_years_forward", n),
            RetryPolicy::BiDirectional { past, future } => {
                write!(f, "bidirectional_{}_back_{}_forward", past, future)
            }
        }
    }
}

impl std::str::FromStr for RetryPolicy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fail_always" => Ok(RetryPolicy::FailAlways),
            s if s.starts_with("try_") && s.ends_with("_years_back") => {
                let n_str = s
                    .strip_prefix("try_")
                    .and_then(|s| s.strip_suffix("_years_back"))
                    .ok_or_else(|| format!("invalid retry policy format: {}", s))?;
                let n = n_str
                    .parse::<u64>()
                    .map_err(|_| format!("invalid year count: {}", n_str))?;
                Ok(RetryPolicy::TryPreviousYears(n))
            }
            s if s.starts_with("try_") && s.ends_with("_years_forward") => {
                let n_str = s
                    .strip_prefix("try_")
                    .and_then(|s| s.strip_suffix("_years_forward"))
                    .ok_or_else(|| format!("invalid retry policy format: {}", s))?;
                let n = n_str
                    .parse::<u64>()
                    .map_err(|_| format!("invalid year count: {}", n_str))?;
                Ok(RetryPolicy::TryNextYears(n))
            }
            s if s.starts_with("bidirectional_") => {
                let rest = s.strip_prefix("bidirectional_").unwrap();
                let parts: Vec<&str> = rest.split("_back_").collect();
                if parts.len() != 2 {
                    return Err(format!("invalid bidirectional format: {}", s));
                }
                let past = parts[0]
                    .parse::<u64>()
                    .map_err(|_| format!("invalid past year count: {}", parts[0]))?;
                let future_str = parts[1]
                    .strip_suffix("_forward")
                    .ok_or_else(|| format!("invalid bidirectional suffix: {}", s))?;
                let future = future_str
                    .parse::<u64>()
                    .map_err(|_| format!("invalid future year count: {}", future_str))?;
                Ok(RetryPolicy::BiDirectional { past, future })
            }
            _ => Err(format!(
                "unknown retry policy: {}. valid options: fail_always, try_N_years_back, try_N_years_forward, bidirectional_N_back_M_forward",
                s
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candidate_years_fail_always() {
        let policy = RetryPolicy::FailAlways;
        assert_eq!(policy.candidate_years(2021), vec![2021]);
    }

    #[test]
    fn test_candidate_years_try_previous() {
        let policy = RetryPolicy::TryPreviousYears(2);
        assert_eq!(policy.candidate_years(2021), vec![2021, 2020, 2019]);
    }

    #[test]
    fn test_candidate_years_try_next() {
        let policy = RetryPolicy::TryNextYears(2);
        assert_eq!(policy.candidate_years(2021), vec![2021, 2022, 2023]);
    }

    #[test]
    fn test_candidate_years_bidirectional() {
        let policy = RetryPolicy::BiDirectional { past: 1, future: 2 };
        assert_eq!(policy.candidate_years(2021), vec![2021, 2020, 2022, 2023]);
    }

    #[test]
    fn test_candidate_years_past_boundary() {
        let policy = RetryPolicy::TryPreviousYears(3);
        // Year 0 should be skipped (base_year < i), but 1999 should be included
        assert_eq!(policy.candidate_years(2002), vec![2002, 2001, 2000, 1999]);
    }

    #[test]
    fn test_parse_fail_always() {
        assert_eq!("fail_always".parse::<RetryPolicy>().unwrap(), RetryPolicy::FailAlways);
    }

    #[test]
    fn test_parse_try_previous_years() {
        let parsed = "try_2_years_back".parse::<RetryPolicy>().unwrap();
        assert_eq!(parsed, RetryPolicy::TryPreviousYears(2));
        assert_eq!(parsed.candidate_years(2021), vec![2021, 2020, 2019]);
    }

    #[test]
    fn test_parse_try_next_years() {
        let parsed = "try_3_years_forward".parse::<RetryPolicy>().unwrap();
        assert_eq!(parsed, RetryPolicy::TryNextYears(3));
        assert_eq!(parsed.candidate_years(2021), vec![2021, 2022, 2023, 2024]);
    }

    #[test]
    fn test_parse_bidirectional() {
        let parsed = "bidirectional_2_back_1_forward".parse::<RetryPolicy>().unwrap();
        assert_eq!(
            parsed,
            RetryPolicy::BiDirectional { past: 2, future: 1 }
        );
        assert_eq!(parsed.candidate_years(2021), vec![2021, 2020, 2019, 2022]);
    }

    #[test]
    fn test_display_fail_always() {
        assert_eq!(RetryPolicy::FailAlways.to_string(), "fail_always");
    }

    #[test]
    fn test_display_try_previous_years() {
        assert_eq!(
            RetryPolicy::TryPreviousYears(2).to_string(),
            "try_2_years_back"
        );
    }

    #[test]
    fn test_display_try_next_years() {
        assert_eq!(
            RetryPolicy::TryNextYears(3).to_string(),
            "try_3_years_forward"
        );
    }

    #[test]
    fn test_display_bidirectional() {
        assert_eq!(
            RetryPolicy::BiDirectional { past: 2, future: 1 }.to_string(),
            "bidirectional_2_back_1_forward"
        );
    }

    #[test]
    fn test_roundtrip_serialize_deserialize() {
        for policy in [
            RetryPolicy::FailAlways,
            RetryPolicy::TryPreviousYears(2),
            RetryPolicy::TryNextYears(3),
            RetryPolicy::BiDirectional { past: 2, future: 1 },
        ] {
            let s = policy.to_string();
            let parsed = s.parse::<RetryPolicy>().unwrap();
            assert_eq!(policy, parsed);
        }
    }
}
