#![allow(dead_code)]
use std::io::prelude::*;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::env;
use std::fs::File;

use url_serde;
use serde_json;
use reqwest::IntoUrl;

use chrono::prelude::*;
use url::Url;

use regex::Regex;

mod api;

#[derive(Debug)]
pub struct Config {
    pub cache_dir: PathBuf,
    pub tag_only_search: bool,
    pub fuzzy_search: bool,
    tags_cache_file: PathBuf,
    pins_cache_file: PathBuf,
}

impl Config {
    pub fn new() -> Result<Self, String> {

        fn get_app_dir() -> PathBuf {
            let mut dir = env::home_dir().unwrap_or_else(|| PathBuf::from(""));
            dir.push(".cache");
            dir.push("rusty-pin");
            dir
        }

        let cache_dir = get_app_dir();
        Config::create_cache_dir(cache_dir).and_then(|cache_dir| {
            Ok(Config {
                tag_only_search: false,
                fuzzy_search: false,
                tags_cache_file: cache_dir.join("tags.cache"),
                pins_cache_file: cache_dir.join("pins.cache"),
                cache_dir,
            })
        })
    }

    pub fn set_cache_dir<P: AsRef<Path>>(&mut self, p: &P) -> Result<(), String> {
        self.cache_dir = Config::create_cache_dir(p)?;
        self.tags_cache_file = self.cache_dir.join("tags.cache");
        self.pins_cache_file = self.cache_dir.join("pins.cache");
        Ok(())
    }

    pub fn enable_tag_only_search(&mut self, v: bool) {
        self.tag_only_search = v;
    }

    pub fn enable_fuzzy_search(&mut self, v: bool) {
        self.fuzzy_search = v;
    }

    fn create_cache_dir<P: AsRef<Path>>(cache_dir: P) -> Result<PathBuf, String> {
        use std::fs;
        fs::create_dir_all(&cache_dir)
            .map_err(|e| e.description().to_owned())
            .and_then(|_| Ok(cache_dir.as_ref().to_path_buf()))
    }
}

#[derive(Debug)]
pub struct Pinboard {
    api: api::Api,
    cfg: Config,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tag(String, usize);

#[derive(Serialize, Deserialize, Debug)]
pub struct Pin {
    #[serde(with = "url_serde", rename = "href")]
    pub url: Url,
    #[serde(rename = "description")]
    pub title: String,
    pub tags: String,
    pub shared: String,
    pub toread: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extended: Option<String>,
    #[serde(default = "Utc::now")]
    pub time: DateTime<Utc>,
    #[serde(skip)]
    meta: Option<String>,
    #[serde(skip)]
    hash: Option<String>,
    #[serde(skip)]
    tag_list: Vec<String>,
}

#[derive(Debug)]
pub struct PinBuilder {
    pin: Pin,
}

impl PinBuilder {
    pub fn new<T: IntoUrl>(url: T, title: String) -> Self {
        let pin = Pin {
            url: url.into_url().unwrap(),
            title,
            time: Utc::now(),
            tags: String::new(),
            shared: String::new(),
            toread: String::new(),
            extended: None,
            meta: None,
            hash: None,
            tag_list: vec![],
        };
        PinBuilder { pin }
    }
}

impl PinBuilder {
    pub fn tags(mut self, t: String) -> Self {
        self.pin.tag_list = t.split_whitespace().map(|s| s.into()).collect();
        self.pin.tags = t;
        self
    }

    pub fn shared(mut self, f: &str) -> Self {
        self.pin.shared = f.to_string();
        self
    }

    pub fn toread(mut self, f: &str) -> Self {
        self.pin.toread = f.to_string();
        self
    }

    pub fn description(mut self, x: String) -> Self {
        self.pin.extended = Some(x);
        self
    }

    pub fn into_pin(self) -> Pin {
        self.pin
    }
}


impl Pin {
    pub fn time(&self) -> DateTime<Utc> {
        self.time
    }

    pub fn contains(&self, query: &str) -> bool {
        let temp;
        let mut q = query;
        if query.chars().any(|c| !c.is_lowercase()) {
            temp = query.to_lowercase();
            q = &temp;
        }

        self.title.to_lowercase().contains(q) ||
            self.tags.to_lowercase().contains(q) ||
            self.url.as_ref().contains(q) ||
            (self.extended.is_some() && self.extended.as_ref().unwrap().to_lowercase().contains(q))
    }

    pub fn contains_fuzzy(&self, re: &Regex) -> bool {
        re.captures(&self.title).is_some() || re.captures(&self.tags).is_some() ||
            re.captures(&self.url.as_ref()).is_some() ||
            (self.extended.is_some() && re.captures(self.extended.as_ref().unwrap()).is_some())
    }
}

impl Pinboard {
    pub fn new(auth_token: String) -> Result<Self, String> {
        let cfg = Config::new()?;
        Ok(Pinboard {
            api: api::Api::new(auth_token),
            cfg,
        })
    }

    pub fn add(self, p: Pin) -> Result<(), String> {
        self.api.add_url(p)
    }

    pub fn search_items(&self, q: &str) -> Result<Option<Vec<Pin>>, String> {
        if self.cfg.pins_cache_file.exists() {
            let cached_pins = self.read_file(&self.cfg.pins_cache_file)?;
            let cached_pins: Vec<Pin> = serde_json::from_str(&cached_pins).map_err(|e| {
                e.description().to_owned()
            })?;

            let r = if !self.cfg.fuzzy_search {
                let q = &q.to_lowercase();
                cached_pins
                    .into_iter()
                    .filter(|item| item.contains(q))
                    .collect::<Vec<Pin>>()
            } else {
                // Build a string for regex: "HAMID" => "H.*A.*M.*I.*D"
                let mut fuzzy_string = q.chars()
                    .map(|c| format!("{}", c))
                    .collect::<Vec<String>>()
                    .join(r".*");
                // Set case-insensitive regex option.
                fuzzy_string.insert_str(0, "(?i)");
                let re = Regex::new(&fuzzy_string).map_err(|_| {
                    "Can't search for given query!".to_owned()
                })?;
                cached_pins
                    .into_iter()
                    .filter(|item| item.contains_fuzzy(&re))
                    .collect::<Vec<Pin>>()
            };
            match r.len() {
                0 => Ok(None),
                _ => Ok(Some(r)),
            }
        } else {
            Err(format!(
                "pins cache file not present: {}",
                self.cfg.pins_cache_file.to_str().unwrap_or("")
            ))
        }
    }

    pub fn search_tags(&self, q: &str) -> Result<Option<Vec<Tag>>, String> {
        if self.cfg.tags_cache_file.exists() {
            let cached_tags = self.read_file(&self.cfg.tags_cache_file)?;
            let cached_tags: Vec<Tag> = serde_json::from_str(&cached_tags).map_err(|e| {
                e.description().to_owned()
            })?;

            let r = if !self.cfg.fuzzy_search {
                let q = &q.to_lowercase();
                cached_tags
                    .into_iter()
                    .filter(|item| item.0.to_lowercase().contains(q))
                    .collect::<Vec<Tag>>()
            } else {
                // Build a string for regex: "HAMID" => "H.*A.*M.*I.*D"
                let mut fuzzy_string = q.chars()
                    .map(|c| format!("{}", c))
                    .collect::<Vec<String>>()
                    .join(r".*");
                // Set case-insensitive regex option.
                fuzzy_string.insert_str(0, "(?i)");
                let re = Regex::new(&fuzzy_string).map_err(|_| {
                    "Can't search for given query!".to_owned()
                })?;
                cached_tags
                    .into_iter()
                    .filter(|item| re.captures(&item.0).is_some())
                    .collect::<Vec<Tag>>()
            };
            match r.len() {
                0 => Ok(None),
                _ => Ok(Some(r)),
            }
        } else {
            Err(format!(
                "tags cache file not present: {}",
                self.cfg.tags_cache_file.to_str().unwrap_or("")
            ))
        }
    }

    pub fn is_cache_outdated(&self, last_update: DateTime<Utc>) -> Result<bool, String> {
        self.api.recent_update().and_then(
            |res| Ok(last_update < res),
        )
    }

    fn update_cache(&self) -> Result<(), String> {
        //TODO: cache all searchable text in lowercase format to make "pin.contains()" efficient.
        // Write all pins
        let mut f = File::create(&self.cfg.pins_cache_file).map_err(|e| {
            e.description().to_owned()
        })?;
        self.api
            .all_pins()
            .and_then(|pins| serde_json::to_vec(&pins).map_err(|e| e.description().to_owned()))
            .and_then(|data| f.write_all(&data).map_err(|e| e.description().to_owned()))?;

        // Write all tags
        let mut f = File::create(&self.cfg.tags_cache_file).map_err(|e| {
            e.description().to_owned()
        })?;
        self.api
            .tags_frequency()
            .and_then(|tags| serde_json::to_vec(&tags).map_err(|e| e.description().to_owned()))
            .and_then(|data| f.write_all(&data).map_err(|e| e.description().to_owned()))
    }
}

/// private implementations
impl Pinboard {
    fn read_file<P: AsRef<Path>>(&self, p: P) -> Result<String, String> {

        File::open(p)
            .map_err(|e| e.description().to_owned())
            .and_then(|mut f| {
                let mut content = String::new();
                f.read_to_string(&mut content)
                    .map_err(|e| e.description().to_owned())
                    .and_then(|_| Ok(content))
            })
    }
}


#[cfg(test)]
mod tests {
    // TODO: Add tests for case insensitivity searches of tags/pins
    use super::*;

    #[test]
    fn test_config() {
        let mut h = env::home_dir().unwrap();
        h.push(".cache");
        h.push("rusty-pin");
        let c = Config::new().expect("Can't initiate 'Config'.");
        assert_eq!(c.cache_dir, h);

        h.push("pins");
        h.set_extension("cache");
        assert_eq!(c.pins_cache_file, h);

        h.set_file_name("tags");
        h.set_extension("cache");
        assert_eq!(c.tags_cache_file, h);
    }

    #[test]
    fn test_set_cache_dir() {
        let mut h = env::home_dir().unwrap();
        let mut c = Config::new().expect("Can't initiate 'Config'.");

        h.push(".cache");
        h.push("rustypin");
        c.set_cache_dir(&h).expect("Can't change cache path.");

        h.push("tags.cache");
        assert_eq!(c.tags_cache_file, h);

        h.set_file_name("pins.cache");
        assert_eq!(c.pins_cache_file, h);
    }

    #[test]
    fn test_builder() {
        let p = PinBuilder::new(
            "https://githuуй.com/Здравствуйт?q=13#fragment",
            "title".to_string(),
        ).tags("tag1 tag2".to_string())
            .into_pin();
        assert_eq!(p.title, "title");
        assert_eq!(
            p.url,
            Url::parse("https://githuуй.com/Здравствуйт?q=13#fragment").unwrap()
        );
        assert_eq!(p.tag_list.len(), 2);
        assert_eq!(p.tags, "tag1 tag2".to_string());
        assert_eq!(p.tag_list, vec!["tag1", "tag2"]);
    }

    #[test]
    fn test_pin_contain() {
        let p = PinBuilder::new(
            "http://правительство.рф",
            "An open source ecosystem for IoT development · PlatformIO".to_string(),
        ).tags("tag1 tag2".to_string())
            .into_pin();

        assert!(p.contains("·"));
        assert!(p.contains("· PlatformIO"));
        assert!(p.contains("IoT"));
        assert!(p.contains("tag"));
        assert!(p.contains("tag1"));
    }

    #[test]
    fn test_search_pins() {
        let mut pinboard = Pinboard::new(include_str!("auth_token.txt").to_string()).unwrap();
        pinboard.cfg.enable_tag_only_search(false);
        pinboard.cfg.enable_fuzzy_search(false);

        // non-fuzzy search
        let pins = pinboard.search_items("rust").unwrap_or_else(|e| panic!(e));
        assert!(pins.is_some());
        // fuzzy search
        pinboard.cfg.enable_fuzzy_search(true);
        let pins = pinboard.search_items("solvingbootp").unwrap_or_else(
            |e| panic!(e),
        );
        assert!(pins.is_some());

        let pins = pinboard.search_items("non-existence-pin").unwrap_or_else(
            |e| panic!(e),
        );
        assert!(pins.is_none());

        // non-fuzzy search
        let pins = pinboard
            .search_items("failure - Cargo: packages for Rust")
            .unwrap_or_else(|e| panic!(e));
        assert!(pins.is_some());
        let pins = pins.unwrap();
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0].url.as_str(), "https://crates.io/crates/failure");

        // fuzzy search
        pinboard.cfg.enable_fuzzy_search(true);
        let pins = pinboard.search_items("failurecargopackage") // "failure cargo package"
            .unwrap_or_else(|e| panic!(e));
        assert!(pins.is_some());
        let pins = pins.unwrap();
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0].url.as_str(), "https://crates.io/crates/failure");
    }

    #[test]
    fn test_search_tags() {
        let mut pinboard = Pinboard::new(include_str!("auth_token.txt").to_string()).unwrap();
        pinboard.cfg.enable_fuzzy_search(false);

        let tags = pinboard.search_tags("django").unwrap_or_else(|e| panic!(e));
        assert!(tags.is_some());

        // non-fuzzy search test
        let tags = pinboard.search_tags("non-existence-tag").unwrap_or_else(
            |e| panic!(e),
        );
        assert!(tags.is_none());
        // fuzzy search test
        pinboard.cfg.enable_fuzzy_search(true);
        let tags = pinboard.search_tags("non-existence-tag").unwrap_or_else(
            |e| panic!(e),
        );
        assert!(tags.is_none());

        // non-fuzzy search test
        let tags = pinboard.search_tags("Lumia920").unwrap_or_else(
            |e| panic!(e),
        );
        assert!(tags.is_some());
        let tags = tags.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].1, 2);
        // fuzzy search test
        pinboard.cfg.enable_fuzzy_search(true);
        let tags = pinboard.search_tags("Lumia920").unwrap_or_else(
            |e| panic!(e),
        );
        assert!(tags.is_some());
        let tags = tags.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].1, 2);

    }

    #[ignore]
    #[test]
    fn test_update_cache() {
        let pinboard = Pinboard::new(include_str!("auth_token.txt").to_string());
        pinboard.unwrap().update_cache().unwrap_or_else(
            |e| panic!(e),
        );
    }
}
