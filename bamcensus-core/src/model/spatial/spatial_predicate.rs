use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// Spatial relationship used to select Census geometries for a query region.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpatialPredicate {
    /// Select geometries that overlap or touch the region.
    #[default]
    Intersects,
    /// Select geometries whose centroid is inside the region.
    CentroidWithin,
    /// Select geometries completely within the region.
    Within,
    /// Select geometries with at least this fraction of their area in the region.
    AreaOverlap { min_fraction: f64 },
}

impl SpatialPredicate {
    /// Validate parameters, including those constructed directly or deserialized.
    pub fn validate(&self) -> Result<(), String> {
        if let Self::AreaOverlap { min_fraction } = self {
            if !min_fraction.is_finite() || *min_fraction <= 0.0 || *min_fraction > 1.0 {
                return Err("area overlap fraction must be finite and in (0, 1]".to_string());
            }
        }
        Ok(())
    }
}

impl fmt::Display for SpatialPredicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Intersects => write!(f, "intersects"),
            Self::CentroidWithin => write!(f, "centroid_within"),
            Self::Within => write!(f, "within"),
            Self::AreaOverlap { min_fraction } => write!(f, "area_overlap:{min_fraction}"),
        }
    }
}

impl FromStr for SpatialPredicate {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let predicate = match value {
            "intersects" => Self::Intersects,
            "centroid_within" | "centroid-within" => Self::CentroidWithin,
            "within" => Self::Within,
            _ => {
                let fraction = value
                    .strip_prefix("area_overlap:")
                    .or_else(|| value.strip_prefix("area-overlap:"))
                    .ok_or_else(|| format!("unknown spatial predicate: {value}"))?;
                let min_fraction = fraction
                    .parse::<f64>()
                    .map_err(|e| format!("invalid area overlap fraction '{fraction}': {e}"))?;
                Self::AreaOverlap { min_fraction }
            }
        };
        predicate.validate()?;
        Ok(predicate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_round_trips_predicates() {
        for (input, expected) in [
            ("intersects", SpatialPredicate::Intersects),
            ("centroid_within", SpatialPredicate::CentroidWithin),
            ("centroid-within", SpatialPredicate::CentroidWithin),
            ("within", SpatialPredicate::Within),
            (
                "area-overlap:0.5",
                SpatialPredicate::AreaOverlap { min_fraction: 0.5 },
            ),
            (
                "area_overlap:1",
                SpatialPredicate::AreaOverlap { min_fraction: 1.0 },
            ),
        ] {
            let predicate = input.parse::<SpatialPredicate>().unwrap();
            assert_eq!(predicate, expected);
            assert_eq!(
                predicate.to_string().parse::<SpatialPredicate>().unwrap(),
                expected
            );
            let json = serde_json::to_string(&predicate).unwrap();
            assert_eq!(
                serde_json::from_str::<SpatialPredicate>(&json).unwrap(),
                expected
            );
        }
        assert_eq!(SpatialPredicate::default(), SpatialPredicate::Intersects);
    }

    #[test]
    fn rejects_invalid_predicates_and_fractions() {
        for input in [
            "unknown",
            "area_overlap",
            "area_overlap:",
            "area_overlap:0",
            "area_overlap:-0.1",
            "area_overlap:1.01",
            "area_overlap:NaN",
            "area_overlap:inf",
        ] {
            assert!(input.parse::<SpatialPredicate>().is_err(), "{input}");
        }
        assert!(SpatialPredicate::AreaOverlap {
            min_fraction: f64::NAN
        }
        .validate()
        .is_err());
        assert!(SpatialPredicate::AreaOverlap { min_fraction: 0.01 }
            .validate()
            .is_ok());
    }
}
