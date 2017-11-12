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
            let mut dir: PathBuf = match env::home_dir() {
                Some(path) => PathBuf::from(path),
                None => PathBuf::from(""),
            };
            dir.push(".cache");
            dir.push("rusty-pin");
            dir
        }

        let cache_dir = get_app_dir();
        let cache_dir = Config::create_cache_dir(cache_dir)?;
        Ok(Config {
            tag_only_search: false,
            fuzzy_search: false,
            tags_cache_file: cache_dir.join("tags.cache"),
            pins_cache_file: cache_dir.join("pins.cache"),
            cache_dir,
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
        if cache_dir.as_ref().exists() {
            Ok(cache_dir.as_ref().to_path_buf())
        } else {
            match fs::create_dir_all(&cache_dir) {
                Err(e) => Err(format!("{}", e)),
                Ok(_) => Ok(PathBuf::from(cache_dir.as_ref())),
            }
        }
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

    pub fn contains(&self, q: &str) -> bool {
        self.title.contains(q) || self.url.as_ref().contains(q) || self.tags.contains(q)
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
        if self.cfg.tags_cache_file.exists() {
            //TODO: To be continued!
        } else {
            return Err(format!(
                "items cache file not present: {}",
                self.cfg.tags_cache_file.to_str().unwrap_or("")
            ));
        }
        Ok(None)
    }

    pub fn search_tags(&self, q: &str) -> Result<Option<Vec<Tag>>, String> {
        let cached_tags = self.read_file(&self.cfg.tags_cache_file)?;

        let cached_tags: Vec<Tag> = match serde_json::from_str(&cached_tags) {
            Ok(cached_tags) => cached_tags,
            Err(e) => return Err(format!("{:?}", e)),
        };

        //TODO: Implement fuzzy search
        let r = if !self.cfg.fuzzy_search {
            let r = cached_tags
                .into_iter()
                .filter(|item| item.0.contains(q))
                .collect::<Vec<Tag>>();
            match r.len() {
                0 => None,
                _ => Some(r),
            }
        } else {
            None
        };
        Ok(r)
    }
}

/// private implementations
impl Pinboard {
    fn read_file<P: AsRef<Path>>(&self, p: P) -> Result<String, String> {
        let f = File::open(p);
        let mut fd = match f {
            Ok(c) => c,
            Err(e) => return Err(format!("{:?}", e.description())),
        };

        let mut content = String::new();
        let r = fd.read_to_string(&mut content);
        if let Err(e) = r {
            Err(format!("{:?}", e.description()))
        } else {
            Ok(content)
        }

    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let mut h = env::home_dir().unwrap();
        h.push(".cache");
        h.push("rusty-pin");
        let c = Config::new().expect("Can't initiate 'Config'.");
        assert_eq!(c.cache_dir, h);
    }

    #[test]
    fn test_set_cache_dir() {
        let mut h = env::home_dir().unwrap();
        h.push(".cache");
        h.push("rusty-pin");
        let mut c = Config::new().expect("Can't initiate 'Config'.");

        h.push("pins");
        h.set_extension("cache");
        println!("{:?}", h);
        assert_eq!(c.pins_cache_file, h);

        h = env::home_dir().unwrap();
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
}
