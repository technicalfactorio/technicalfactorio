//! Application to quickly index megabases.

use std::cmp::Ordering;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::path::Path;
use std::path::PathBuf;
use std::io;
use std::io::Read;
use std::io::stdin;
use std::fs::File;
use serde::Deserialize;
use serde::Serialize;
use std::io::Write;

use sha2::Digest;

use directories::BaseDirs;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Megabases {
    saves: Vec<MegabaseMetadata>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct MegabaseMetadata {
    /// The filename of this megabase.
    name: String,
    /// The post/video showcasing the megabase.
    source_link: String,
    /// The version of Factorio this save was saved with.
    factorio_version: FactorioVersion,
    /// The hex-encoded String of the sha256 hash of this Savefile.
    sha256: String,
    /// The mirror of this save hosted by /u/mulark, if permitted by the map author.
    #[serde(skip_serializing_if = "Option::is_none")]
    download_link_mirror: Option<String>,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Ord)]
#[serde(into = "String")]
#[serde(try_from = "&str")]
pub struct FactorioVersion {
    major: u16,
    minor: u16,
    patch: u16,
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

fn main() -> io::Result<()> {
    loop {
        let mut metadata = MegabaseMetadata::default();
        let mut s = String::new();
        println!("Enter a base source link for the post describing the megabase.");
        stdin().read_line(&mut s)?;
        check_url_alive(&s)?;
        s = s.trim().to_string();
        metadata.source_link = s.clone();
        s.clear();
        println!("Enter the name of the savefile (.zip not required nor forbidden)");
        stdin().read_line(&mut s)?;
        s = s.trim().to_string();

        let filename = s.replace("'", "");
        let filepath = if !filename.ends_with(".zip") {
            PathBuf::from(format!("{}.zip", filename))
        } else {
            PathBuf::from(filename)
        };
        metadata.name = filepath.file_name().unwrap().to_string_lossy().to_string();
        let savefile_path = find_savefile(&filepath);
        s.clear();
        let jh = {
            let savefile_path = savefile_path.clone();
            std::thread::spawn(move || {
                sha256sum(&savefile_path)
            })
        };
        metadata.factorio_version = run_factorio_and_find_savefile_version(&savefile_path);
        metadata.sha256 = jh.join().unwrap();
        eprintln!("{:#?}", metadata);
        eprintln!("Name|Link|Factorio Version|sha256");
        eprintln!("{}|{}|{}|{}",
            metadata.name,
            metadata.source_link,
            metadata.factorio_version.to_string(),
            metadata.sha256
        );
        let megabases_json = PathBuf::from("megabases.json");
        if megabases_json.exists() {
            let s = std::fs::read_to_string(&megabases_json)?;
            let mut megabases = serde_json::from_str::<Megabases>(&s)?;
            megabases.saves.push(metadata);
            megabases.saves.sort();
            megabases.saves.dedup();
            let s = serde_json::to_string_pretty(&megabases)?;
            write!(std::fs::File::create(&megabases_json)?, "{}", s)?;
        } else {
            let megabases = Megabases {
                saves: vec![metadata],
            };
            let s = serde_json::to_string_pretty(&megabases)?;
            write!(std::fs::File::create(megabases_json)?, "{}", s)?;
        }
    }
}

pub fn sha256sum(file_path: &PathBuf) -> String {
    let mut f = File::open(file_path).unwrap();
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    format!("{:x}", sha2::Sha256::digest(&buf))
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

fn find_savefile(path: &PathBuf) -> PathBuf {
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

/// Attempts to look for the Factorio Executable, panics if it can't find it
fn factorio_exe() -> PathBuf {
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

/// Tests if a url is available
fn check_url_alive(url: &str) -> io::Result<()> {
    let resp = ureq::head(url).call();
    if resp.status() == 200 {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, format!("{:?}", resp)))
    }
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization_deserialization() {
        let metadata =
        MegabaseMetadata {
            name: "0.17-0.18 Poobers Beautiful Base.zip".to_owned(),
            source_link: "https://www.youtube.com/watch?v=hMrxuaIdeeE".to_owned(),
            factorio_version: FactorioVersion {
                major: 0,
                minor: 18,
                patch: 17,
            },
            sha256: "e8346b825adb2059de4710e1aa9431f97fb40026c375b0de8ea126a5f8b254f4".to_owned(),
            download_link_mirror: None,
        };
        let all = Megabases {
            saves: vec![metadata],
        };
        let as_str = serde_json::to_string(&all).unwrap();
        let _t = serde_json::from_str::<Megabases>(&as_str).unwrap();
    }
}
