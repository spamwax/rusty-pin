use std::borrow::Cow;
use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};

use reqwest::IntoUrl;
use rmps::{Deserializer, Serializer};
use serde::Deserialize;

use chrono::prelude::*;
use url::Url;

use failure::Error;

use regex::Regex;

use env_logger;

mod api;
mod cached_data;
mod config;

#[cfg(test)]
mod mockito_helper;

pub mod pin;
pub mod tag;

use self::cached_data::*;
use self::config::Config;

pub use self::pin::{Pin, PinBuilder};
pub use self::tag::{Tag, TagFreq};

#[derive(Debug)]
pub struct Pinboard<'api, 'pin> {
    api: api::Api<'api>,
    cfg: Config,
    cached_data: CachedData<'pin>,
}

impl<'api, 'pin> Pinboard<'api, 'pin> {
    pub fn new<S, P>(auth_token: S, cached_dir: Option<P>) -> Result<Self, Error>
    where
        S: Into<Cow<'api, str>>,
        P: AsRef<Path>,
    {
        let _ = env_logger::try_init();
        let api = api::Api::new(auth_token);
        let cfg = Config::new();

        debug!("pinb::new: calling CachedData::new");
        let mut cached_data = CachedData::new(cached_dir)?;
        if !cached_data.cache_ok() {
            debug!("pinb::new: cache file missing, calling update");
            cached_data.update_cache(&api)?;
            debug!("pinb::new:   update done.");
        } else {
            debug!("pinb::new: cache not missing");
        }

        let pinboard = Pinboard {
            api,
            cfg,
            cached_data,
        };
        Ok(pinboard)
    }

    pub fn set_cache_dir<P: AsRef<Path>>(&mut self, p: &P) -> Result<(), Error> {
        debug!("set_cache_dir: starting.");
        self.cached_data.set_cache_dir(p)?;
        self.cached_data.load_cache_data_from_file()
    }

    pub fn enable_tag_only_search(&mut self, v: bool) {
        debug!("enable_tag_only_search: starting.");
        self.cfg.tag_only_search = v;
    }

    pub fn enable_fuzzy_search(&mut self, v: bool) {
        debug!("enable_fuzzy_search: starting.");
        self.cfg.fuzzy_search = v;
    }

    pub fn enable_private_new_pin(&mut self, v: bool) {
        debug!("enable_private_new_pin: starting.");
        self.cfg.private_new_pin = v;
    }

    pub fn enable_toread_new_pin(&mut self, v: bool) {
        debug!("enable_toread_new_pin: starting.");
        self.cfg.toread_new_pin = v;
    }

    pub fn add_pin(&self, p: Pin) -> Result<(), Error> {
        debug!("add_pin: starting.");
        self.api.add_url(p)
    }

    pub fn delete<T: IntoUrl>(&self, url: T) -> Result<(), Error> {
        debug!("delete: starting.");
        self.api.delete(url)
    }

    pub fn is_cache_outdated(&self, last_update: DateTime<Utc>) -> Result<bool, Error> {
        debug!("is_cache_outdated: starting.");
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
impl<'api, 'pin> Pinboard<'api, 'pin> {
    /// Searches all the fields within bookmarks to filter them.
    /// This function honors [pinboard::config::Config] settings for fuzzy search & tag_only search.
    pub fn search_items(&self, query: &str) -> Result<Option<Vec<&Pin>>, Error> {
        debug!("search_items: starting.");
        if self.cached_data.cache_ok() {
            let r = if !self.cfg.fuzzy_search {
                let q = &query.to_lowercase();
                self.cached_data
                    .pins
                    .as_ref()
                    .map(|p| {
                        p.iter()
                            .filter(|item: &&CachedPin| {
                                if self.cfg.tag_only_search {
                                    item.pin.tag_contains(q, None)
                                } else {
                                    item.pin.contains(q)
                                }
                            })
                            .map(|item| &item.pin)
                            .collect::<Vec<&Pin>>()
                    })
                    .unwrap_or(Vec::new())
            } else {
                // Build a string for regex: "HAMID" => "H.*A.*M.*I.*D"
                let mut fuzzy_string = query
                    .chars()
                    .map(|c| c.to_string())
                    .collect::<Vec<String>>()
                    .join(r".*");
                // Set case-insensitive regex option.
                fuzzy_string.insert_str(0, "(?i)");
                let re = Regex::new(&fuzzy_string)?;
                self.cached_data
                    .pins
                    .as_ref()
                    .map(|p| {
                        p.iter()
                            .filter(|item| {
                                if self.cfg.tag_only_search {
                                    item.pin.tag_contains("", Some(&re))
                                } else {
                                    item.pin.contains_fuzzy(&re)
                                }
                            })
                            .map(|item| &item.pin)
                            .collect::<Vec<&Pin>>()
                    })
                    .unwrap_or(Vec::new())
            };
            match r.len() {
                0 => Ok(None),
                _ => Ok(Some(r)),
            }
        } else {
            bail!("Tags cache data is invalid")
        }
    }

    /// Only looks up q within list of cached tags.
    /// This function honors [pinboard::config::Config] settings for fuzzy search.
    pub fn search_list_of_tags(&self, query: &str) -> Result<Option<Vec<&Tag>>, Error> {
        debug!("search_list_of_tags: starting.");
        if self.cached_data.cache_ok() {
            let r = if !self.cfg.fuzzy_search {
                let q = &query.to_lowercase();
                self.cached_data
                    .tags
                    .as_ref()
                    .map(|t| {
                        t.into_iter()
                            .filter(|item| item.0.to_lowercase().contains(q))
                            .collect::<Vec<&Tag>>()
                    })
                    .unwrap_or(Vec::new())
            } else {
                // Build a string for regex: "HAMID" => "H.*A.*M.*I.*D"
                let mut fuzzy_string = query
                    .chars()
                    .map(|c| format!("{}", c))
                    .collect::<Vec<String>>()
                    .join(r".*");
                // Set case-insensitive regex option.
                fuzzy_string.insert_str(0, "(?i)");
                let re = Regex::new(&fuzzy_string)?;
                // .map_err(|_| "Can't search for given query!".to_owned())?;
                self.cached_data
                    .tags
                    .as_ref()
                    .map(|t| {
                        t.into_iter()
                            .filter(|item| re.is_match(&item.0))
                            .collect::<Vec<&Tag>>()
                    })
                    .unwrap_or(Vec::new())
            };
            match r.len() {
                0 => Ok(None),
                _ => Ok(Some(r)),
            }
        } else {
            bail!("Tags cache data is invalid")
        }
    }

    /// Searches the selected `fields` within bookmarks to filter them.
    /// This function honors [pinboard::config::Config] settings for fuzzy search only.
    pub fn search<'b, I, S>(
        &self,
        q: &'b I,
        fields: &[SearchType],
    ) -> Result<Option<Vec<&Pin>>, Error>
    where
        &'b I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        debug!("search: starting.");
        if !self.cached_data.cache_ok() {
            bail!("Cache data is invalid.");
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
                .map(|p| {
                    p.into_iter()
                        .filter(|cached_pin: &&CachedPin| {
                            q.into_iter().all(|s| {
                                let query = &s.as_ref().to_lowercase();
                                search_fields.iter().any(|search_type| match *search_type {
                                    SearchType::TitleOnly => cached_pin.pin.title.contains(query),
                                    SearchType::TagOnly => {
                                        cached_pin.tag_list.iter().any(|tag| tag.contains(query))
                                    }
                                    SearchType::UrlOnly => {
                                        cached_pin.pin.url.as_ref().contains(query)
                                    }
                                    SearchType::DescriptionOnly => {
                                        if let Some(ref extended) = cached_pin.pin.extended {
                                            extended.contains(query)
                                        } else {
                                            false
                                        }
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
                })
                .unwrap_or(Vec::new())
        } else {
            let regex_queries = q.into_iter()
                .map(|s| {
                    let query = &s.as_ref().to_lowercase();
                    // Build a string for regex: "HAMID" => "H.*A.*M.*I.*D"
                    let mut fuzzy_string = String::with_capacity(query.len() * query.len() * 2);
                    fuzzy_string.extend(
                        query
                            .chars()
                            .map(|c| c.to_string())
                            .collect::<Vec<String>>()
                            .join(r".*")
                            .chars(),
                    );
                    // Set case-insensitive regex option.
                    let mut fuzzy_regex: String = String::with_capacity(fuzzy_string.len() + 2);
                    fuzzy_regex.extend("(?i)".chars().chain(fuzzy_string.chars()));
                    Regex::new(&fuzzy_string)
                        .map_err(|e| format!("{:?}", e))
                        .expect("Couldn't build regex using given search query")
                })
                .collect::<Vec<Regex>>();
            self.cached_data
                .pins
                .as_ref()
                .map(|p| {
                    p.into_iter()
                        .filter(|cached_pin: &&CachedPin| {
                            regex_queries.iter().all(|re| {
                                search_fields.iter().any(|search_type| match *search_type {
                                    SearchType::TitleOnly => re.is_match(&cached_pin.pin.title),
                                    SearchType::TagOnly => {
                                        cached_pin.tag_list.iter().any(|t| re.is_match(t))
                                    }
                                    SearchType::UrlOnly => re.is_match(cached_pin.pin.url.as_ref()),
                                    SearchType::DescriptionOnly => {
                                        if let Some(ref extended) = cached_pin.pin.extended {
                                            re.is_match(extended)
                                        } else {
                                            false
                                        }
                                    }
                                    SearchType::TagTitleOnly => {
                                        re.is_match(&cached_pin.pin.title)
                                            || cached_pin.tag_list.iter().any(|t| re.is_match(t))
                                    }
                                })
                            })
                        })
                        .map(|p| &p.pin)
                        .collect::<Vec<&Pin>>()
                })
                .unwrap_or(Vec::new())
        };

        match results.len() {
            0 => Ok(None),
            _ => Ok(Some(results)),
        }
    }

    /// Update local cache
    pub fn update_cache(&mut self) -> Result<(), Error> {
        debug!("update_cache: starting.");
        self.cached_data.update_cache(&self.api)
    }

    /// Returns list of all Tags (tag, frequency)
    pub fn list_tag_pairs(&self) -> &Option<Vec<Tag>> {
        debug!("list_tag_pairs: starting.");
        &self.cached_data.tags
    }

    /// Returns list of all bookmarks
    pub fn list_bookmarks(&self) -> Option<Vec<&Pin>> {
        debug!("list_bookmarks: starting.");
        self.cached_data
            .pins
            .as_ref()
            .map(|v| v.iter().map(|p| &p.pin).collect())
    }

    /// Suggest a list of tags based on the provided URL
    pub fn popular_tags<T: IntoUrl>(&self, url: T) -> Result<Vec<String>, Error> {
        debug!("popular_tags: starting.");
        self.api.suggest_tags(url)
    }
}

#[cfg(test)]
mod tests {
    // TODO: Add tests for case insensitivity searches of tags/pins
    use super::*;
    use std::env;
    use std::fs;

    #[cfg(feature = "bench")]
    use test::Bencher;

    use self::mockito_helper::create_mockito_servers;
    use mockito::{mock, Matcher};
    use url;

    #[test]
    fn test_cached_data() {
        let _ = env_logger::try_init();
        debug!("test_cached_data: starting.");
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
        let _ = env_logger::try_init();
        debug!("test_set_cache_dir: starting.");
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
        let _ = env_logger::try_init();
        debug!("test_search_items: starting.");
        let (_m1, _m2) = create_mockito_servers();
        let mut _home = env::home_dir().unwrap();
        _home.push(".cache");
        _home.push("mockito-rusty-pin");
        let cache_path = Some(_home);

        let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path).unwrap();
        pinboard.enable_fuzzy_search(false);

        {
            let pins = pinboard
                .search_items("openpgp")
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
                .search_items("gemhobi")
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
            assert_eq!(1, pins.unwrap().len());
        }
    }

    #[test]
    fn search_tag_pairs() {
        let _ = env_logger::try_init();
        debug!("search_tag_pairs: starting.");
        let (_m1, _m2) = create_mockito_servers();
        let mut _home = env::home_dir().unwrap();
        _home.push(".cache");
        _home.push("mockito-rusty-pin");
        let cache_path = Some(_home);

        let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path).unwrap();
        pinboard.enable_fuzzy_search(false);

        {
            let tags = pinboard
                .search_list_of_tags("ctags")
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
                .search_list_of_tags("yubikey")
                .unwrap_or_else(|e| panic!(e));
            assert!(tags.is_some());
            let tags = tags.unwrap();
            assert_eq!(tags.len(), 1);
            assert_eq!(TagFreq::Used(3), tags[0].1);
        }

        {
            // fuzzy search test
            pinboard.enable_fuzzy_search(true);
            let tags = pinboard
                .search_list_of_tags("mroooe")
                .unwrap_or_else(|e| panic!(e));
            assert!(tags.is_some());
            let tags = tags.unwrap();
            assert_eq!(1, tags.len());
            assert_eq!(TagFreq::Used(5), tags[0].1);
        }

        {
            // empty query non-fuzzy
            pinboard.enable_fuzzy_search(false);
            let tags = pinboard
                .search_list_of_tags("")
                .unwrap_or_else(|e| panic!(e));
            assert!(tags.is_some());
            assert_eq!(94, tags.unwrap().len());
        }

        {
            // empty query fuzzy
            pinboard.enable_fuzzy_search(true);
            let tags = pinboard
                .search_list_of_tags("")
                .unwrap_or_else(|e| panic!(e));
            assert!(tags.is_some());
            assert_eq!(94, tags.unwrap().len());
        }
    }

    #[test]
    fn list_tags() {
        let _ = env_logger::try_init();
        debug!("list_tags: starting.");
        let (_m1, _m2) = create_mockito_servers();
        let mut _home = env::home_dir().unwrap();
        _home.push(".cache");
        _home.push("mockito-rusty-pin");
        let _ = fs::remove_file(&_home);
        let cache_path = Some(_home);

        let pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path).unwrap();
        assert!(pinboard.list_tag_pairs().is_some());
    }

    #[test]
    fn list_bookmarks() {
        let _ = env_logger::try_init();
        debug!("list_bookmarks: starting.");
        let (_m1, _m2) = create_mockito_servers();
        let mut _home = env::home_dir().expect("Can't find home dir");
        _home.push(".cache");
        _home.push("mockito-rusty-pin");
        let cache_path = Some(_home);

        let pinboard =
            Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");
        assert!(pinboard.list_bookmarks().is_some());
    }

    #[test]
    fn popular_tags() {
        let _ = env_logger::try_init();
        debug!("popular_tags: starting.");
        let _m1 = mock("GET", Matcher::Regex(r"^/posts/suggest.*$".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"popular":["datetime","library","rust"]},{"recommended":["datetime","library","programming","rust"]}]"#)
            .create();
        let mut _home = env::home_dir().expect("Can't get home_dir");
        _home.push(".cache");
        _home.push("mockito-rusty-pin");
        let cache_path = Some(_home);

        let pinboard =
            Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");
        let tags = pinboard.popular_tags("https://docs.rs/chrono/0.4.0/chrono");
        assert!(tags.is_ok());
        let tags = tags.unwrap();
        assert!(tags.len() >= 2);

        // Test invalid URL
        let error = pinboard
            .popular_tags("docs.rs/chrono/0.4.0/chrono")
            .expect_err("Suggested tags for malformed url");
        assert_eq!(
            &url::ParseError::RelativeUrlWithoutBase,
            error
                .root_cause()
                .downcast_ref::<url::ParseError>()
                .unwrap()
        );
    }

    #[test]
    fn search_multi_query_multi_field() {
        let _ = env_logger::try_init();
        debug!("search_multi_query_multi_field: starting.");
        let (_m1, _m2) = create_mockito_servers();
        let mut _home = env::home_dir().unwrap();
        _home.push(".cache");
        _home.push("mockito-rusty-pin");
        let cache_path = Some(_home);

        let mut pinboard =
            Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");
        // Find pins that have all keywords almost anywhere
        {
            pinboard.enable_fuzzy_search(false);
            let queries = ["eagle", "design", "fun"];
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
            let queries = ["eagle", "design", "fun"];
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
            let queries = ["pixlas"];
            let fields = vec![SearchType::UrlOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert_eq!(1, pins.as_ref().unwrap().len());
        }

        // Fuzzy search
        {
            pinboard.enable_fuzzy_search(true);
            let queries = ["rust", "strange", "cross", "readme", "hint"];
            let fields = vec![
                SearchType::TitleOnly,
                SearchType::TagOnly,
                SearchType::DescriptionOnly,
                SearchType::UrlOnly,
            ];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert_eq!(3, pins.as_ref().unwrap().len());
        }

        // Fuzzy search unicode
        {
            pinboard.enable_fuzzy_search(true);
            let queries = ["\u{0622}\u{0645}\u{0648}\u{0632}\u{0634}\u{06cc}"]; // آموزشی
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
            assert_eq!(3, pins.as_ref().unwrap().len());
        }

        // Tag-only search
        {
            pinboard.enable_fuzzy_search(false);
            let queries = ["rust"];
            let fields = vec![SearchType::TagOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert_eq!(10, pins.as_ref().unwrap().len());

            let queries = ["keyboard", "hacks"];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
            assert_eq!(1, pins.as_ref().unwrap().len());
        }

        // Tag-only search with fuzzy search
        {
            pinboard.enable_fuzzy_search(true);
            let queries = ["backup"];
            let fields = vec![SearchType::TagOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert_eq!(2, pins.as_ref().unwrap().len());
        }

        // title+url search non-fuzzy
        {
            pinboard.enable_fuzzy_search(false);
            let queries = ["世", "macos"];
            let fields = vec![SearchType::TitleOnly, SearchType::UrlOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert_eq!(1, pins.as_ref().unwrap().len());
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

    #[test]
    fn serde_update_cache() {
        let _ = env_logger::try_init();
        debug!("serde_update_cache: starting.");
        let (_m1, _m2) = create_mockito_servers();
        let mut _home = env::home_dir().unwrap();
        _home.push(".cache");
        _home.push("mockito-rusty-pin");
        let cache_path = Some(_home);

        let p = Pinboard::new(include_str!("api_token.txt"), cache_path);
        let mut pinboard = p.unwrap_or_else(|e| panic!("{:?}", e));

        // Get all pins directly from Pinboard.in (no caching)
        let fresh_pins = pinboard.api.all_pins().unwrap();

        let _ = pinboard.update_cache().expect("Couldn't update the cache");

        let cached_pins = pinboard.list_bookmarks().unwrap();
        assert_eq!(fresh_pins.len(), cached_pins.len());

        // Pick 3 pins and compare them between cached dataset and freshly fetched from Pinboard's
        // API
        for idx in [0u32, 10u32, 50u32].iter() {
            debug!("serde_update_cache: Checking pin[{}]", idx);
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

    // I am not sure why I wrote this test as it is kind of similar to serde_update_cache
    #[test]
    fn test_update_cache() {
        let _ = env_logger::try_init();
        debug!("test_update_cache: starting.");

        const IDX: usize = 25;

        let (_m1, _m2) = create_mockito_servers();
        let mut _home = env::home_dir().unwrap();
        _home.push(".cache");
        _home.push("mockito-rusty-pin");

        let cache_path = Some(_home.clone());

        debug!("Running first update_cache");

        // First remove all folders to force a full update
        let _ = fs::remove_dir_all(_home).expect("Can't remove dir to prepare the test");

        // Pinboard::new() will call update_cache since we remove the cache folder.
        let pb = Pinboard::new(include_str!("api_token.txt"), cache_path);
        let mut pinboard = match pb {
            Ok(v) => v,
            Err(e) => panic!("{:?}", e),
        };
        let pins = match pinboard.cached_data.pins.take() {
            Some(v) => v,
            None => panic!("No pins found in cache!"),
        };
        let tags = match pinboard.cached_data.tags.take() {
            Some(v) => v,
            None => panic!("No tags found in cache!"),
        };
        assert!(pins.len() > IDX);
        assert!(tags.len() > IDX);

        debug!("Running second update_cache");
        pinboard
            .cached_data
            .update_cache(&pinboard.api)
            .unwrap_or_else(|e| panic!(e));
        pinboard
            .cached_data
            .load_cache_data_from_file()
            .unwrap_or_else(|e| panic!(e));
        assert!(pinboard.cached_data.cache_ok());

        assert!(pinboard.cached_data.pins.is_some());
        debug!(
            "{:?}\n\n{:?}\n\n",
            pins[IDX],
            pinboard.cached_data.pins.as_ref().unwrap()[IDX]
        );
        assert_eq!(pins[IDX], pinboard.cached_data.pins.as_ref().unwrap()[IDX]);
        assert_eq!(
            pins.len(),
            pinboard.cached_data.pins.as_ref().unwrap().len()
        );

        assert!(pinboard.cached_data.tags.is_some());
        debug!(
            "{:?}\n{:?}",
            tags[IDX],
            pinboard.cached_data.tags.as_ref().unwrap()[IDX]
        );
        assert_eq!(
            tags.len(),
            pinboard.cached_data.tags.as_ref().unwrap().len()
        );
        assert_eq!(tags[IDX], pinboard.cached_data.tags.as_ref().unwrap()[IDX]);
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn bench_search_items_openpgp(b: &mut Bencher) {
        let _ = env_logger::try_init();
        debug!("bench_search_items_non_fuzzy: starting.");
        let (_m1, _m2) = create_mockito_servers();
        let mut _home = env::home_dir().unwrap();
        _home.push(".cache");
        _home.push("mockito-rusty-pin");
        let cache_path = Some(_home);

        let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path).unwrap();
        pinboard.enable_fuzzy_search(false);
        pinboard.enable_tag_only_search(false);
        let query = "openpgp";
        b.iter(|| {
            let _ = pinboard.search_items(query).unwrap_or_else(|e| panic!(e));
        })
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn bench_search_openpgp(b: &mut Bencher) {
        let _ = env_logger::try_init();
        debug!("bench_search_openpgp: starting.");
        let (_m1, _m2) = create_mockito_servers();
        let mut _home = env::home_dir().unwrap();
        _home.push(".cache");
        _home.push("mockito-rusty-pin");
        let cache_path = Some(_home);

        let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path).unwrap();
        pinboard.enable_fuzzy_search(false);
        pinboard.enable_tag_only_search(false);
        let queries = ["openpgp"];
        let fields = vec![
            SearchType::TitleOnly,
            SearchType::TagOnly,
            SearchType::UrlOnly,
            SearchType::DescriptionOnly,
        ];
        b.iter(|| {
            let _pins = pinboard
                .search(&queries, fields.as_slice())
                .unwrap_or_else(|e| panic!(e));
        });
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn bench_search_non_fuzzy(b: &mut Bencher) {
        let _ = env_logger::try_init();
        debug!("bench_search_non_fuzzy: starting.");
        let (_m1, _m2) = create_mockito_servers();
        let mut _home = env::home_dir().unwrap();
        _home.push(".cache");
        _home.push("mockito-rusty-pin");
        let cache_path = Some(_home);

        let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path).unwrap();
        pinboard.enable_fuzzy_search(false);
        let queries = ["zfs", "fr"];
        let fields = vec![];
        b.iter(|| {
            let _pins = pinboard
                .search(&queries, fields.as_slice())
                .unwrap_or_else(|e| panic!(e));
        });
    }

    #[cfg(feature = "bench")]
    #[bench]
    fn bench_search_fuzzy(b: &mut Bencher) {
        let _ = env_logger::try_init();
        debug!("bench_search_fuzzy: starting.");
        let (_m1, _m2) = create_mockito_servers();
        let mut _home = env::home_dir().unwrap();
        _home.push(".cache");
        _home.push("mockito-rusty-pin");
        let cache_path = Some(_home);

        let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path).unwrap();
        pinboard.enable_fuzzy_search(true);
        let queries = ["zfs", "fr"];
        let fields = vec![];
        b.iter(|| {
            let _pins = pinboard
                .search(&queries, fields.as_slice())
                .unwrap_or_else(|e| panic!(e));
        });
    }
}
