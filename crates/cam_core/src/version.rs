//! Version strings as the server and the addon releases write them, compared as semver.

/// Lenient semver parse: leading `v` stripped, missing minor/patch padded (`1.2` -> `1.2.0`).
pub fn parse_semver(v: &str) -> Option<semver::Version> {
    let v = v.trim().trim_start_matches(['v', 'V']);
    if let Ok(v) = semver::Version::parse(v) {
        return Some(v);
    }
    let (core, rest) = match v.find(['-', '+']) {
        Some(i) => (&v[..i], &v[i..]),
        None => (v, ""),
    };
    let mut parts: Vec<&str> = core.split('.').collect();
    if parts.is_empty()
        || parts.len() > 3
        || parts.iter().any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    while parts.len() < 3 {
        parts.push("0");
    }
    semver::Version::parse(&format!("{}{rest}", parts.join("."))).ok()
}

/// True when version `a` is greater than `b`, false when either side is not a version.
pub fn semver_gt(a: &str, b: &str) -> bool {
    matches!((parse_semver(a), parse_semver(b)), (Some(a), Some(b)) if a > b)
}

/// How far a release is from the installed one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bump {
    Major,
    Minor,
    Patch,
    PreRelease,
    Republished,
}

impl Bump {
    pub fn label(self) -> &'static str {
        match self {
            Bump::Major => "Major update",
            Bump::Minor => "Minor update",
            Bump::Patch => "Patch",
            Bump::PreRelease => "Pre-release",
            Bump::Republished => "Re-published",
        }
    }
}

/// Parses a version tag strictly: strips leading `=`, `v` or `V` and surrounding whitespace. `None` unless the rest is
/// a full `x.y.z[-pre]` version.
fn parse_tag(version: &str) -> Option<semver::Version> {
    let v = version.trim().trim_start_matches(['=', 'v', 'V']).trim();
    semver::Version::parse(v).ok()
}

/// Classifies the jump between two tags, or `None` when they are not comparable versions.
pub fn release_bump(from: &str, to: &str) -> Option<Bump> {
    if from == to {
        return Some(Bump::Republished);
    }
    let (installed, available) = (parse_tag(from)?, parse_tag(to)?);
    if available <= installed {
        return None;
    }
    if !installed.pre.is_empty() || !available.pre.is_empty() {
        return Some(Bump::PreRelease);
    }
    Some(if installed.major != available.major {
        Bump::Major
    } else if installed.minor != available.minor {
        Bump::Minor
    } else {
        Bump::Patch
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_compare() {
        assert!(semver_gt("1.10.0", "1.9.9"));
        assert!(semver_gt("v2.0", "1.9.9"));
        assert!(!semver_gt("1.0.0", "1.0.0"));
        assert!(!semver_gt("garbage", "1.0.0"));
    }

    #[test]
    fn bump_classification() {
        assert_eq!(release_bump("1.2.3", "1.2.3"), Some(Bump::Republished));
        assert_eq!(release_bump("1.2.3", "2.0.0"), Some(Bump::Major));
        assert_eq!(release_bump("v1.2.3", "v1.3.0"), Some(Bump::Minor));
        assert_eq!(release_bump("1.2.3", "1.2.4"), Some(Bump::Patch));
        assert_eq!(release_bump("1.2.3", "1.3.0-beta.1"), Some(Bump::PreRelease));
        assert_eq!(release_bump("1.2.3-rc.1", "1.2.3"), Some(Bump::PreRelease));
        assert_eq!(release_bump("1.2.3", "1.2.2"), None);
        assert_eq!(release_bump("2026-10-01", "2026-10-03"), None);
        assert_eq!(release_bump("0.6", "0.7"), None);
        assert_eq!(Bump::Minor.label(), "Minor update");
    }
}
