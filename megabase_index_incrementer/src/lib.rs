use std::convert::TryInto;
use std::path::PathBuf;
use std::io::Read;
use sha2::digest::Digest;
use std::fs::File;
use std::path::Path;
use serde::Deserialize;
use serde::Serialize;
use directories::BaseDirs;

use std::cmp::Ordering;
use std::convert::TryFrom;

pub(crate) fn sha256sum(file_path: &Path) -> String {
    let mut f = File::open(file_path).unwrap();
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    format!("{:x}", sha2::Sha256::digest(&buf))
}


fn find_savefile(path: &Path) -> PathBuf {
    if path.is_absolute() && path.exists() {
        return path.to_owned();
    }
    let rw_saves = find_factorio_rw_dir().join("saves");
    let mut maybe_path = rw_saves.join(&path);
    if maybe_path.exists() {
        maybe_path
    } else {
        eprintln!("Could not find file {:?}. Searched {:?}, trying current dir",
            path, maybe_path);
        if let Ok(path) = std::env::current_dir() {
            maybe_path = path.join(&path);
            if maybe_path.exists() {
                return maybe_path;
            }
        }
        panic!("Could not find file in current dir");
    }
}


fn find_factorio_install() -> PathBuf {
    let possible_install = if cfg!(target_os = "linux") {
        let base_dir = BaseDirs::new().unwrap();
        base_dir
            .home_dir()
            .join(".local")
            .join("share")
            .join("Steam")
            .join("steamapps")
            .join("common")
            .join("Factorio")
            .join("")
    } else {
        PathBuf::from("C:\\")
            .join("Program Files (x86)")
            .join("Steam")
            .join("steamapps")
            .join("common")
            .join("Factorio")
            .join("")
    };
    if possible_install.exists() {
        println!("Found steam version installed");
        possible_install
    } else {
        unimplemented!("Could not find Factorio install, only looked for steam version");
    }
}

fn find_factorio_rw_dir() -> PathBuf {
    let cfg_path = find_factorio_install().join("config-path.cfg");
    let mut use_system_rw_directories = true;
    if cfg_path.exists() {
        let cfg_file = std::fs::read_to_string(&cfg_path).unwrap();
        for line in cfg_file.lines() {
            if line.starts_with("use-system-read-write-data-directories=") {
                let val = line.split('=').nth(1).unwrap();
                use_system_rw_directories = val.parse::<bool>().unwrap_or(true);
                break;
            }
        }
    }
    if use_system_rw_directories {
        if cfg!(target_os = "linux") {
            // ~/.factorio/
            BaseDirs::new()
                .unwrap()
                .home_dir()
                .join(".factorio")
                .join("")
        } else {
            // %appdata%\Roaming\
            BaseDirs::new()
                .unwrap()
                .data_dir()
                .join("Factorio")
                .join("")
        }
    } else {
        // Probably local install
        find_factorio_install()
    }
}

/// Attempts to look for the Factorio Executable, panics if it can't find it
pub(crate) fn factorio_exe() -> PathBuf {
    let p = if cfg!(target_os = "linux") {
        find_factorio_install().join("bin").join("x64").join("factorio")
    } else {
        find_factorio_install().join("bin").join("x64").join("factorio.exe")
    };
    if p.exists() {
        return p;
    }
    panic!("Could not find Factorio Executable!");
}


fn run_factorio_and_find_savefile_version(savefile: &Path) -> FactorioVersion {
    println!("Determining saved Factorio version");
    let out = std::process::Command::new(factorio_exe())
        .arg("--benchmark")
        .arg(savefile)
        .arg("--benchmark-ticks")
        .arg(1.to_string())
        .arg("--benchmark-runs")
        .arg(1.to_string())
        .output();
    let stdout = String::from_utf8(out.unwrap().stdout).unwrap().replace("\r", "");
    for line in stdout.lines().rev() {
        if line.contains("Map version ") {
            // get rid of everything before the version
            let trim_begin = line.split("Map version ").nth(1).unwrap();
            let version_str = trim_begin.split('-').next().unwrap();
            if let Ok(version) = version_str.try_into() {
                return version;
            }
        }
    }
    panic!("Could not determine version for savefile!");
}

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

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Ord, Hash)]
#[serde(into = "String")]
#[serde(try_from = "&str")]
pub struct FactorioVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl FactorioVersion {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
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

/// Populates the automated portion of the megabase metadata
pub fn populate_metadata(path: &Path) -> Result<MegabaseMetadata, Box<dyn std::error::Error>> {
    let mut metadata = MegabaseMetadata::default();
    metadata.name = path.file_name().unwrap().to_string_lossy().to_string();
    let savefile_path = find_savefile(&path);
    let jh = {
        let savefile_path = savefile_path.clone();
        std::thread::spawn(move || {
            sha256sum(&savefile_path)
        })
    };
    metadata.factorio_version = run_factorio_and_find_savefile_version(&savefile_path);
    metadata.sha256 = jh.join().unwrap();
    Ok(metadata)
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
