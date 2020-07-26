use serde::Deserialize;
use serde::Serialize;

use std::cmp::Ordering;
use std::convert::TryFrom;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Megabases {
    pub saves: Vec<MegabaseMetadata>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct MegabaseMetadata {
    /// The filename of this megabase.
    pub name: String,
    pub author: Option<String>,
    /// The post/video showcasing the megabase.
    pub source_link: String,
    /// The version of Factorio this save was saved with.
    pub factorio_version: FactorioVersion,
    /// The hex-encoded String of the sha256 hash of this Savefile.
    pub sha256: String,
    /// The mirror of this save hosted by /u/mulark, if permitted by the map author.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_link_mirror: Option<String>,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Ord)]
#[serde(into = "String")]
#[serde(try_from = "&str")]
pub struct FactorioVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl FactorioVersion {
    pub fn new(major: u16, minor: u16, patch: u16) -> Self {
        FactorioVersion {
            major,
            minor,
            patch,
        }
    }
}

impl ToString for FactorioVersion {
    fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl From<FactorioVersion> for String {
    fn from(fv: FactorioVersion) -> Self {
        fv.to_string()
    }
}

impl TryFrom<&str> for FactorioVersion {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let splits = s.split('.').collect::<Vec<_>>();
        if splits.len() != 3 {
            return Err("Incorrect number of periods present in version string".to_owned());
        }
        let splits = splits.iter().map(|x| x.parse()).collect::<Vec<_>>();
        if splits.iter().all(|x| x.is_ok()) {
            let splits = splits
                .iter()
                .map(|x| *(x.as_ref().unwrap()))
                .collect::<Vec<_>>();
            Ok(FactorioVersion {
                major: splits[0],
                minor: splits[1],
                patch: splits[2],
            })
        } else {
            Err("Unparseable/non-numeric data found within version subsection!".to_owned())
        }
    }
}

impl PartialOrd for FactorioVersion {
    fn partial_cmp(&self, other: &FactorioVersion) -> Option<Ordering> {
        if self.major > other.major {
            Some(Ordering::Greater)
        } else if self.major < other.major {
            Some(Ordering::Less)
        } else if self.minor > other.minor {
            Some(Ordering::Greater)
        } else if self.minor < other.minor {
            Some(Ordering::Less)
        } else if self.patch > other.patch {
            Some(Ordering::Greater)
        } else if self.patch < other.patch {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_ser_fv() {
        let fv = FactorioVersion {
            major: 0,
            minor: 17,
            patch: 79,
        };
        let serialized = serde_json::to_string(&fv).unwrap();
        assert_eq!(serialized, "\"0.17.79\"");
    }

    #[test]
    fn test_deser_fv() {
        let fv = serde_json::from_str::<FactorioVersion>("\"0.17.79\"").unwrap();
        let fv_reference = FactorioVersion {
            major: 0,
            minor: 17,
            patch: 79,
        };
        assert_eq!(fv, fv_reference);
    }

    #[test]
    fn test_ser_fv_deser_fv_roundtrip() {
        let fv = FactorioVersion {
            major: 0,
            minor: 17,
            patch: 79,
        };
        let serialized = serde_json::to_string(&fv).unwrap();
        let deserialized = serde_json::from_str::<FactorioVersion>(&serialized).unwrap();
        assert_eq!(deserialized, fv);
    }
}
