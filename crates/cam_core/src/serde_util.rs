//! serde helpers for JSON in the shapes the Go client reads and writes.
use serde::{Deserialize, Deserializer};

/// Reads `null` as the type's default, as Go decodes `null` into a zero value.
pub fn null_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

/// An optional timestamp in the form the Go client reads and writes: it has no `null` for times and writes the
/// zero time (0001-01-01T00:00:00Z) for an unset one. Use with `#[serde(default, with = "go_time")]`.
///
/// Reading gives `None` for `null`, a missing field and any time at or before the Unix epoch, which covers the
/// zero time. Writing gives the zero time for `None`, so the Go client can read the file.
pub mod go_time {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(time: &Option<DateTime<Utc>>, s: S) -> Result<S::Ok, S::Error> {
        time.unwrap_or_else(|| Utc.with_ymd_and_hms(1, 1, 1, 0, 0, 0).unwrap()).serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<DateTime<Utc>>, D::Error> {
        Ok(Option::<DateTime<Utc>>::deserialize(d)?.filter(|t| t.timestamp_millis() > 0))
    }
}
