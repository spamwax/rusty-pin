#![allow(dead_code)]
use std::io::prelude::*;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::env;
use std::fs::File;
use std::borrow::Cow;
use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use rmps::{Deserializer, Serializer};

use chrono::prelude::*;
use url::Url;

use regex::Regex;

mod api;
mod config;
mod cached_data;
pub mod pin;

use self::config::Config;
use self::cached_data::*;

pub use self::pin::{Pin, PinBuilder, Tag};

const DEBUG: bool = false;
fn logme(from: &str, msg: &str) {
    if DEBUG {
        println!("** In {:?}: {:?}", from, msg);
    }
}

#[derive(Debug)]
pub struct Pinboard<'a> {
    api: api::Api<'a>,
    cfg: Config,
    cached_data: CachedData,
}

impl<'a> Pinboard<'a> {
    pub fn new<S, P>(auth_token: S, cached_dir: Option<P>) -> Result<Self, String>
    where
        S: Into<Cow<'a, str>>,
        P: AsRef<Path>,
    {
        let api = api::Api::new(auth_token);
        let cfg = Config::new();
        let mut cached_data = CachedData::new(cached_dir)?;
        if !cached_data.cache_ok() {
            logme("pinb::new", "cache file missing, calling update");
            cached_data.update_cache(&api)?;
            logme("pinb::new", "  update done.");
        } else {
            logme("pinb::new", "cache not missing");
        }

        let pinboard = Pinboard {
            api: api,
            cfg,
            cached_data,
        };
        Ok(pinboard)
    }

    pub fn set_cache_dir<P: AsRef<Path>>(&mut self, p: &P) -> Result<(), String> {
        self.cached_data.set_cache_dir(p)?;
        self.cached_data.load_cache_data_from_file()
    }

    pub fn enable_tag_only_search(&mut self, v: bool) {
        self.cfg.tag_only_search = v;
    }

    pub fn enable_fuzzy_search(&mut self, v: bool) {
        self.cfg.fuzzy_search = v;
    }

    pub fn enable_private_new_pin(&mut self, v: bool) {
        self.cfg.private_new_pin = v;
    }

    pub fn enable_toread_new_pin(&mut self, v: bool) {
        self.cfg.toread_new_pin = v;
    }

    pub fn add_pin(self, p: Pin) -> Result<(), String> {
        self.api.add_url(p)
    }

    pub fn is_cache_outdated(&self, last_update: DateTime<Utc>) -> Result<bool, String> {
        self.api
            .recent_update()
            .and_then(|res| Ok(last_update < res))
    }
}

pub enum SearchType {
    TitleOnly,
    TagOnly,
    UrlOnly,
    DescriptionOnly,
    TagTitleOnly,
}

// Search functions
impl<'a> Pinboard<'a> {
    /// Searches all the fields within bookmarks to filter them.
    /// This function honors [pinboard::config::Config] settings for fuzzy search & tag_only search.
    pub fn search_items(&self, q: &str) -> Result<Option<Vec<&Pin>>, String> {
        if self.cached_data.cache_ok() {
            let r = if !self.cfg.fuzzy_search {
                let q = &q.to_lowercase();
                self.cached_data
                    .pins
                    .as_ref()
                    .unwrap()
                    .into_iter()
                    .filter(|item| if self.cfg.tag_only_search {
                        item.pin.tag_contains(q, None)
                    } else {
                        item.pin.contains(q)
                    })
                    .map(|item| &item.pin)
                    .collect::<Vec<&Pin>>()
            } else {
                // Build a string for regex: "HAMID" => "H.*A.*M.*I.*D"
                let mut fuzzy_string = q.chars()
                    .map(|c| c.to_string())
                    .collect::<Vec<String>>()
                    .join(r".*");
                // Set case-insensitive regex option.
                fuzzy_string.insert_str(0, "(?i)");
                let re = Regex::new(&fuzzy_string)
                    .map_err(|_| "Can't search for given query!".to_owned())?;
                self.cached_data
                    .pins
                    .as_ref()
                    .unwrap()
                    .into_iter()
                    .filter(|item| if self.cfg.tag_only_search {
                        item.pin.tag_contains("", Some(&re))
                    } else {
                        item.pin.contains_fuzzy(&re)
                    })
                    .map(|item| &item.pin)
                    .collect::<Vec<&Pin>>()
            };
            match r.len() {
                0 => Ok(None),
                _ => Ok(Some(r)),
            }
        } else {
            Err("Tags cache data is invalid.".to_string())
        }
    }

    /// Only looks up q within list of cached tags.
    /// This function honors [pinboard::config::Config] settings for fuzzy search.
    pub fn search_list_of_tags(&self, q: &str) -> Result<Option<Vec<&Tag>>, String> {
        if self.cached_data.cache_ok() {
            let r = if !self.cfg.fuzzy_search {
                let q = &q.to_lowercase();
                self.cached_data
                    .tags
                    .as_ref()
                    .unwrap()
                    .into_iter()
                    .filter(|item| item.0.to_lowercase().contains(q))
                    .collect::<Vec<&Tag>>()
            } else {
                // Build a string for regex: "HAMID" => "H.*A.*M.*I.*D"
                let mut fuzzy_string = q.chars()
                    .map(|c| format!("{}", c))
                    .collect::<Vec<String>>()
                    .join(r".*");
                // Set case-insensitive regex option.
                fuzzy_string.insert_str(0, "(?i)");
                let re = Regex::new(&fuzzy_string)
                    .map_err(|_| "Can't search for given query!".to_owned())?;
                self.cached_data
                    .tags
                    .as_ref()
                    .unwrap()
                    .into_iter()
                    .filter(|item| re.captures(&item.0).is_some())
                    .collect::<Vec<&Tag>>()
            };
            match r.len() {
                0 => Ok(None),
                _ => Ok(Some(r)),
            }
        } else {
            Err("Tags cache data is invalid.".to_string())
        }
    }

    /// Searches the selected `fields` within bookmarks to filter them.
    /// This function honors [pinboard::config::Config] settings for fuzzy search only.
    pub fn search<'b, I, S>(
        &self,
        q: &'b I,
        fields: &[SearchType],
    ) -> Result<Option<Vec<&Pin>>, String>
    where
        &'b I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        if !self.cached_data.cache_ok() {
            return Err(String::from("Cache data is invalid."));
        }
        // When no field is specified, search everywhere
        let all_fields = vec![
            SearchType::TitleOnly,
            SearchType::TagOnly,
            SearchType::UrlOnly,
            SearchType::DescriptionOnly,
        ];
        let search_fields = if fields.is_empty() {
            all_fields.as_slice()
        } else {
            fields
        };

        let results = if !self.cfg.fuzzy_search {
            self.cached_data
                .pins
                .as_ref()
                .unwrap()
                .into_iter()
                .filter(|cached_pin: &&CachedPin| {
                    q.into_iter().all(|s| {
                        let query = &s.as_ref().to_lowercase();
                        search_fields.iter().any(|search_type| match *search_type {
                            SearchType::TitleOnly => cached_pin.pin.title.contains(query),
                            SearchType::TagOnly => cached_pin.tag_list.contains(query),
                            SearchType::UrlOnly => cached_pin.pin.url.as_ref().contains(query),
                            SearchType::DescriptionOnly => {
                                cached_pin.pin.extended.is_some()
                                    && cached_pin.pin.extended.as_ref().unwrap().contains(query)
                            }
                            SearchType::TagTitleOnly => {
                                cached_pin.pin.title.contains(query)
                                    || cached_pin.tag_list.contains(query)
                            }
                        })
                    })
                })
                .map(|p| &p.pin)
                .collect::<Vec<&Pin>>()
        } else {
            let regex_queries = q.into_iter()
                .map(|s| {
                    let query = &s.as_ref().to_lowercase();
                    // Build a string for regex: "HAMID" => "H.*A.*M.*I.*D"
                    let mut fuzzy_string = query
                        .chars()
                        .map(|c| c.to_string())
                        .collect::<Vec<String>>()
                        .join(r".*");
                    // Set case-insensitive regex option.
                    fuzzy_string = "(?i)".chars().chain(fuzzy_string.chars()).collect();
                    Regex::new(&fuzzy_string)
                        .map_err(|_| "Can't search for given query!".to_owned())
                        .expect("Couldn't build regex using given search query!")
                })
                .collect::<Vec<Regex>>();
            self.cached_data
                .pins
                .as_ref()
                .unwrap()
                .into_iter()
                .filter(|cached_pin: &&CachedPin| {
                    regex_queries.iter().all(|re| {
                        search_fields.iter().any(|search_type| match *search_type {
                            SearchType::TitleOnly => re.captures(&cached_pin.pin.title).is_some(),
                            SearchType::TagOnly => {
                                cached_pin.tag_list.iter().any(|t| re.captures(t).is_some())
                            }
                            SearchType::UrlOnly => {
                                re.captures(cached_pin.pin.url.as_ref()).is_some()
                            }
                            SearchType::DescriptionOnly => {
                                cached_pin.pin.extended.is_some()
                                    && re.captures(cached_pin.pin.extended.as_ref().unwrap())
                                        .is_some()
                            }
                            SearchType::TagTitleOnly => {
                                re.captures(&cached_pin.pin.title).is_some()
                                    || cached_pin.tag_list.iter().any(|t| re.captures(t).is_some())
                            }
                        })
                    })
                })
                .map(|p| &p.pin)
                .collect::<Vec<&Pin>>()
        };

        match results.len() {
            0 => Ok(None),
            _ => Ok(Some(results)),
        }
    }

    /// Update local cache
    pub fn update_cache(&mut self) -> Result<(), String> {
        self.cached_data.update_cache(&self.api)
    }

    /// Returns list of all Tags (tag, frequency)
    pub fn list_tag_pairs(&self) -> &Option<Vec<Tag>> {
        &self.cached_data.tags
    }

    /// Returns list of all bookmarks
    pub fn list_bookmarks(&self) -> Option<Vec<&Pin>> {
        self.cached_data
            .pins
            .as_ref()
            .map(|v| v.iter().map(|p| &p.pin).collect())
    }
}

#[cfg(test)]
mod tests {
    // TODO: Add tests for case insensitivity searches of tags/pins
    use super::*;
    use test::Bencher;

    #[test]
    fn test_cached_data() {
        let mut h = env::home_dir().unwrap();
        h.push(".cache");
        h.push("rusty-pin");
        let p: Option<PathBuf> = None;
        let c = CachedData::new(p).expect("Can't initiate 'CachedData'.");
        assert_eq!(c.cache_dir, h);

        // const TAGS_CACHE_FN: &str = "tags.cache";
        // const PINS_CACHE_FN: &str = "pins.cache";
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
        let p: Option<PathBuf> = None;
        let mut c = CachedData::new(p).expect("Can't initiate 'CachedData'.");

        h.push(".cache");
        h.push("rusty-pin");
        c.set_cache_dir(&h).expect("Can't change cache path.");

        h.push("tags.cache");
        assert_eq!(c.tags_cache_file, h);

        h.set_file_name("pins.cache");
        assert_eq!(c.pins_cache_file, h);
    }

    #[test]
    fn test_search_items() {
        let p: Option<PathBuf> = None;
        let mut pinboard = Pinboard::new(include_str!("auth_token.txt"), p).unwrap();
        pinboard.enable_fuzzy_search(false);

        {
            let pins = pinboard
                .search_items("django")
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
        }

        {
            // non-fuzzy search test
            let pins = pinboard
                .search_items("non-existence-tag")
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_none());
        }
        {
            // fuzzy search test
            pinboard.enable_fuzzy_search(true);
            let pins = pinboard
                .search_items("funkYoumoth")
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
            assert_eq!(1, pins.unwrap().len());
        }
    }

    #[test]
    fn search_tag_pairs() {
        let p: Option<PathBuf> = None;
        let mut pinboard = Pinboard::new(include_str!("auth_token.txt"), p).unwrap();
        pinboard.enable_fuzzy_search(false);

        {
            let tags = pinboard
                .search_list_of_tags("django")
                .unwrap_or_else(|e| panic!(e));
            assert!(tags.is_some());
        }

        {
            // non-fuzzy search test
            let tags = pinboard
                .search_list_of_tags("non-existence-tag")
                .unwrap_or_else(|e| panic!(e));
            assert!(tags.is_none());
        }
        {
            // fuzzy search test
            pinboard.enable_fuzzy_search(true);
            let tags = pinboard
                .search_list_of_tags("non-existence-tag")
                .unwrap_or_else(|e| panic!(e));
            assert!(tags.is_none());
        }

        {
            // non-fuzzy search test
            let tags = pinboard
                .search_list_of_tags("Lumia920")
                .unwrap_or_else(|e| panic!(e));
            assert!(tags.is_some());
            let tags = tags.unwrap();
            assert_eq!(tags.len(), 1);
            assert_eq!(tags[0].1, 2);
        }

        {
            // fuzzy search test
            pinboard.enable_fuzzy_search(true);
            let tags = pinboard
                .search_list_of_tags("Lumia920")
                .unwrap_or_else(|e| panic!(e));
            assert!(tags.is_some());
            let tags = tags.unwrap();
            assert_eq!(tags.len(), 1);
            assert_eq!(tags[0].1, 2);
        }
    }

    #[test]
    fn list_tags() {
        let p: Option<PathBuf> = None;
        let pinboard = Pinboard::new(include_str!("auth_token.txt"), p);
        // println!("{:?}", pinboard);
        assert!(pinboard.unwrap().list_tag_pairs().is_some());
    }

    #[test]
    fn list_bookmarks() {
        let p: Option<PathBuf> = None;
        let pinboard = Pinboard::new(include_str!("auth_token.txt"), p);
        assert!(pinboard.unwrap().list_bookmarks().is_some());
    }

    #[test]
    fn search_multi_query_multi_field() {
        let p: Option<PathBuf> = None;
        let mut pinboard = Pinboard::new(include_str!("auth_token.txt"), p).unwrap();
        // Find pins that have all keywords almost anywhere
        {
            pinboard.enable_fuzzy_search(false);
            let queries = ["rust", "python"];
            let fields = vec![
                SearchType::TitleOnly,
                SearchType::TagOnly,
                SearchType::DescriptionOnly,
            ];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());

            // Run same query, this time with Vec<String> instead of Vec<&str>
            let queries = [String::from("rust"), String::from("python")];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
        }

        // Find pins that have all keywords only in Title
        {
            let fields = vec![SearchType::TitleOnly];
            let queries = ["rust", "python"];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_none());
        }

        // Find pins that have all keywords only in Url
        {
            let queries = ["bashy"];
            let fields = vec![SearchType::UrlOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert_eq!(2, pins.as_ref().unwrap().len());
        }

        // Fuzzy search
        {
            pinboard.enable_fuzzy_search(true);
            let queries = [
                "rust", "python", "open", "handoff", "sony", "writing", "elseif", "osx"
            ];
            let fields = vec![
                SearchType::TitleOnly,
                SearchType::TagOnly,
                SearchType::DescriptionOnly,
                SearchType::UrlOnly,
            ];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert_eq!(9, pins.as_ref().unwrap().len());
        }

        // Fuzzy search unicode
        {
            pinboard.enable_fuzzy_search(true);
            let queries = ["世"];
            let fields = vec![
                SearchType::TitleOnly,
                SearchType::TagOnly,
                SearchType::DescriptionOnly,
                SearchType::UrlOnly,
            ];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert_eq!(1, pins.as_ref().unwrap().len());
        }
        // Tag-only search
        {
            pinboard.enable_fuzzy_search(false);
            let queries = ["bestpractices"];
            let fields = vec![SearchType::TagOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert_eq!(3, pins.as_ref().unwrap().len());
        }

        // Tag-only search with fuzzy search
        {
            pinboard.enable_fuzzy_search(true);
            let queries = ["bestpractices"];
            let fields = vec![SearchType::TagOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert_eq!(5, pins.as_ref().unwrap().len());
        }

        // title+url search non-fuzzy
        {
            pinboard.enable_fuzzy_search(false);
            let queries = ["000", "intel"];
            let fields = vec![SearchType::TitleOnly, SearchType::UrlOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert_eq!(2, pins.as_ref().unwrap().len());
        }

        // empty search query
        {
            pinboard.enable_fuzzy_search(false);
            let queries = [""];
            let fields = vec![SearchType::TitleOnly, SearchType::UrlOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
        }
    }

    #[bench]
    fn bench_search_non_fuzzy(b: &mut Bencher) {
        let p: Option<PathBuf> = None;
        let mut pinboard = Pinboard::new(include_str!("auth_token.txt"), p).unwrap();
        pinboard.enable_fuzzy_search(false);
        let queries = ["zfs", "fr"];
        let fields = vec![];
        b.iter(|| {
            let _pins = pinboard
                .search(&queries, fields.as_slice())
                .unwrap_or_else(|e| panic!(e));
        });
    }

    #[bench]
    fn bench_search_fuzzy(b: &mut Bencher) {
        let p: Option<PathBuf> = None;
        let mut pinboard = Pinboard::new(include_str!("auth_token.txt"), p).unwrap();
        pinboard.enable_fuzzy_search(true);
        let queries = ["zfs", "fr"];
        let fields = vec![];
        b.iter(|| {
            let _pins = pinboard
                .search(&queries, fields.as_slice())
                .unwrap_or_else(|e| panic!(e));
        });
    }

    #[test]
    fn serde_update_cache() {
        let p: Option<PathBuf> = None;
        let pinboard = Pinboard::new(include_str!("auth_token.txt"), p);
        let pinboard = pinboard.unwrap();

        // Get all pins directly from Pinboard.in (no caching)
        let fresh_pins = pinboard.api.all_pins().unwrap();

        let cached_pins = pinboard.list_bookmarks().unwrap();
        assert_eq!(
            fresh_pins.len(),
            pinboard.cached_data.pins.as_ref().unwrap().len()
        );

        // Pick 3 pins and compare them between cached dataset and freshly fetched from Pinboard's
        // API
        for idx in [0u32, 10u32, 100u32].iter() {
            println!(" Checking pin[{}]...", idx);
            assert_eq!(
                fresh_pins[*idx as usize].title.to_lowercase(),
                cached_pins[*idx as usize].title.to_lowercase()
            );
            assert_eq!(
                fresh_pins[*idx as usize].url,
                cached_pins[*idx as usize].url
            );
            assert_eq!(
                fresh_pins[*idx as usize].tags.to_lowercase(),
                cached_pins[*idx as usize].tags.to_lowercase()
            );
            assert_eq!(
                fresh_pins[*idx as usize].shared.to_lowercase(),
                cached_pins[*idx as usize].shared.to_lowercase()
            );
            assert_eq!(
                fresh_pins[*idx as usize].toread.to_lowercase(),
                cached_pins[*idx as usize].toread.to_lowercase()
            );
            assert_eq!(
                fresh_pins[*idx as usize].time,
                cached_pins[*idx as usize].time
            );

            if fresh_pins[*idx as usize].extended.is_some() {
                assert!(cached_pins[*idx as usize].extended.is_some());
                assert_eq!(
                    fresh_pins[*idx as usize]
                        .extended
                        .as_ref()
                        .unwrap()
                        .to_lowercase(),
                    cached_pins[*idx as usize]
                        .extended
                        .as_ref()
                        .unwrap()
                        .to_lowercase()
                );
            } else {
                assert!(cached_pins[*idx as usize].extended.is_none());
            }
        }
    }

    #[ignore]
    #[test]
    fn test_update_cache() {
        use std::{thread, time};
        use std::fs;

        let five_secs = time::Duration::from_secs(5);

        // First remove all folders to force a full update
        let mut dir = env::home_dir().unwrap_or_else(|| PathBuf::from(""));
        dir.push(".cache");
        dir.push("rusty-pin");
        fs::remove_dir_all(dir);

        thread::sleep(five_secs);
        println!("Running first update_cache");

        // Pinboard::new() will call update_cache since we remove the cache folder.
        let p: Option<PathBuf> = None;
        let pinboard = Pinboard::new(include_str!("auth_token.txt"), p);
        let mut pinboard = pinboard.unwrap();
        let pins = pinboard.cached_data.pins.take().unwrap();
        let tags = pinboard.cached_data.tags.take().unwrap();

        thread::sleep(five_secs);

        println!("Running second update_cache");
        // pinboard.cached_data.update_cache(&pinboard.api).unwrap_or_else(|e| panic!(e));
        pinboard
            .cached_data
            .load_cache_data_from_file()
            .unwrap_or_else(|e| panic!(e));
        assert!(pinboard.cached_data.cache_ok());

        assert!(pinboard.cached_data.pins.is_some());
        println!(
            "{:?}\n\n{:?}\n\n",
            pins[20],
            pinboard.cached_data.pins.as_ref().unwrap()[20]
        );
        assert_eq!(pins[20], pinboard.cached_data.pins.as_ref().unwrap()[20]);
        assert_eq!(
            pins.len(),
            pinboard.cached_data.pins.as_ref().unwrap().len()
        );

        assert!(pinboard.cached_data.tags.is_some());
        println!(
            "{:?}\n{:?}",
            tags[20],
            pinboard.cached_data.tags.as_ref().unwrap()[20]
        );
        assert_eq!(tags[20], pinboard.cached_data.tags.as_ref().unwrap()[20]);
        assert_eq!(
            tags.len(),
            pinboard.cached_data.tags.as_ref().unwrap().len()
        );
    }
}
