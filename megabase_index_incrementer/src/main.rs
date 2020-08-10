//! Application to quickly index megabases.

use crate::lib::populate_metadata;
use std::path::PathBuf;
use std::io::stdin;

use std::io::Write;


mod lib;
use lib::MegabaseMetadata;
use lib::Megabases;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    loop {
        upgrade_existing_metadatas().unwrap();
        let mut s = String::new();
        println!("Enter a base source link for the post describing the megabase.");
        stdin().read_line(&mut s)?;
        let author = Some(check_url_alive_get_author(&s).unwrap());
        s = s.trim().to_string();
        let source_link = s.clone();
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
        let mut metadata = populate_metadata(&filepath)?;

        metadata.author = author;
        metadata.source_link = source_link;

        eprintln!("{:#?}", metadata);
        eprintln!("Name|Link|Factorio Version|sha256");
        eprintln!("{}|{}|{}|{}",
            metadata.name,
            metadata.source_link,
            metadata.factorio_version.to_string(),
            metadata.sha256
        );
        write_metadata_to_disk(metadata).unwrap();
    }
}

fn write_metadata_to_disk(metadata: MegabaseMetadata) -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}

/// Removes an existing metadata from the json where the sha256sum matches the provided sha256.
fn remove_metadata_from_disk_with_sha256(sha256: &str) -> Result<(), Box<dyn std::error::Error>> {
    let megabases_json = PathBuf::from("megabases.json");
    if megabases_json.exists() {
        let s = std::fs::read_to_string(&megabases_json)?;
        let mut megabases = serde_json::from_str::<Megabases>(&s)?;
        megabases.saves.retain(|metadata| metadata.sha256 != sha256);
        let s = serde_json::to_string_pretty(&megabases)?;
        write!(std::fs::File::create(&megabases_json)?, "{}", s)?;
    }
    Ok(())
}

fn upgrade_existing_metadatas() -> Result<(), Box<dyn std::error::Error>> {
    println!("Checking to upgrade existing metadata");
    let megabases_json = PathBuf::from("megabases.json");
    let s = std::fs::read_to_string(&megabases_json)?;
    let megabases = serde_json::from_str::<Megabases>(&s)?;
    for mut metadata in megabases.saves {
        if metadata.author.is_none() {
            println!("Upgrading {}", metadata.name);
            println!("{}", metadata.source_link);
            metadata.author = Some(check_url_alive_get_author(&metadata.source_link).unwrap());
            remove_metadata_from_disk_with_sha256(&metadata.sha256)?;
            write_metadata_to_disk(metadata.clone())?;
        }
        if let Some(author) = &metadata.author {
            if author.starts_with("/user/") {
                metadata.author = Some("https://www.reddit.com".to_owned() + author);
                remove_metadata_from_disk_with_sha256(&metadata.sha256)?;
                write_metadata_to_disk(metadata.clone())?;
            }
        }
    }
    Ok(())
}

/// Tests if a url is available
fn check_url_alive_get_author(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    if url.contains("reddit.com") {
        let r = ureq::get(&url).call().into_string()?;
        if r.contains("\"author\":") {
            let re = regex::Regex::new("\"author\":\"([^\"]+)\"").unwrap();
            let matches = re.captures_iter(&r);
            for cap in matches {
                let cap = format!("/user/{}/", &cap[1]);
                println!("Found post to be submitted by {}, press enter if this is correct, \"n\" to see next user"
                    , cap);
                let mut s = String::new();
                stdin().read_line(&mut s)?;
                s = s.trim().to_owned();
                if s.is_empty() {
                    return Ok(cap);
                }
            }
            panic!("Didn't find any user");
        } else {
            let re = regex::Regex::new("/u/([^/\"\x20]+)[/\"\x20]").unwrap();
            let matches = re.captures_iter(&r);
            for cap in matches {
                let cap = format!("/user/{}/", &cap[1]);
                println!("Found post to be submitted by {}, press enter if this is correct, \"n\" to see next user"
                    , cap);
                let mut s = String::new();
                stdin().read_line(&mut s)?;
                s = s.trim().to_owned();
                if s.is_empty() {
                    return Ok(cap);
                }
            }
            panic!("Didn't find any user");
        }
    } else {
        let r = ureq::head(url).call();
        if r.status() == 200 {
            println!("Please enter the user who created the save");
            let mut s = String::new();
            stdin().read_line(&mut s)?;
            s = s.trim().to_owned();
            if !s.is_empty() {
                Ok(s)
            } else {
                panic!("Author is required");
            }
        } else {
            panic!("Status code error for URL {}", url);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lib::FactorioVersion;
    use super::*;

    #[test]
    fn test_serialization_deserialization() {
        let metadata =
        MegabaseMetadata {
            name: "0.17-0.18 Poobers Beautiful Base.zip".to_owned(),
            author: None,
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

    #[test]
    fn test_deser_file() {
        let p = PathBuf::from("megabases.json");
        if p.exists() {
            let s = std::fs::read_to_string(&p).unwrap();
            serde_json::from_str::<Megabases>(&s).unwrap();
        }
    }
}
