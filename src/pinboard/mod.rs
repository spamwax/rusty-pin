use std::borrow::Cow;
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::rmps::Serializer;
// use rmps::{Deserializer, Serializer};
// use serde::Deserialize;

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
#[cfg(test)]
mod tests;

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
        let _ = Url::parse(&p.url)?;
        self.api.add_url(p)
    }

    pub fn delete<T: AsRef<str>>(&self, url: T) -> Result<(), Error> {
        debug!("delete: starting.");
        self.api.delete(url)
    }

    pub fn is_cache_outdated(&self, last_update: DateTime<Utc>) -> Result<bool, Error> {
        debug!("is_cache_outdated: starting.");
        self.api
            .recent_update()
            .and_then(|res| Ok(last_update < res))
    }

    /// Delete a tag
    pub fn delete_tag<T: AsRef<str>>(&self, tag: T) -> Result<(), Error> {
        debug!("delete_tag: starting.");
        self.api.tag_delete(tag)
    }

    /// Rename a tag
    pub fn rename_tag<T: AsRef<str>>(&self, old: T, new: T) -> Result<(), Error> {
        debug!("rename_tag: starting.");
        self.api.tag_rename(old, new)
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
    pub fn popular_tags<T: AsRef<str>>(&self, url: T) -> Result<Vec<String>, Error> {
        debug!("popular_tags: starting.");
        let _ = Url::parse(url.as_ref())?;
        self.api.suggest_tags(url)
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
                    .unwrap_or_default()
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
                    .unwrap_or_default()
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
                        t.iter()
                            .filter(|item| item.0.to_lowercase().contains(q))
                            .collect::<Vec<&Tag>>()
                    })
                    .unwrap_or_default()
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
                self.cached_data
                    .tags
                    .as_ref()
                    .map(|t| {
                        t.iter()
                            .filter(|item| re.is_match(&item.0))
                            .collect::<Vec<&Tag>>()
                    })
                    .unwrap_or_default()
            };
            match r.len() {
                0 => Ok(None),
                _ => Ok(Some(r)),
            }
        } else {
            bail!("Tags cache data is invalid")
        }
    }

    // TODO: find_url should return pins that match `q` barring their fragment //
    // https://github.com/sharkdp/hexyl#preview  and
    // https://github.com/sharkdp/hexyl
    // should be considered identical (?!)

    /// Finds all pins whose url is an exact match of the `q`
    ///
    /// find_url("http://google.com/public") will match following
    /// http://google.com/public
    /// but not following
    /// http://google.com/public#fragment
    pub fn find_url<S>(&'pin self, q: S) -> Result<Option<Vec<&'pin Pin<'pin>>>, Error>
    where
        S: AsRef<str>,
    {
        debug!("find_url: starting.");
        if !self.cached_data.cache_ok() {
            bail!("Cache data is invalid.");
        }
        let query = &q.as_ref().to_lowercase();
        let results = self
            .cached_data
            .pins
            .as_ref()
            .map(|p: &Vec<CachedPin<'pin>>| {
                p.iter()
                    .filter(|cached_pin: &&CachedPin<'pin>| {
                        cached_pin.pin.url.to_lowercase().as_str() == query
                    })
                    .map(|p| &p.pin)
                    .collect::<Vec<&'pin Pin>>()
            })
            .unwrap_or_default();
        match results.len() {
            0 => Ok(None),
            _ => Ok(Some(results)),
        }
    }

    /// Searches the selected `fields` within bookmarks to filter them.
    /// This function honors [pinboard::config::Config] settings for fuzzy search only.
    pub fn search<'b, I, S>(
        &'pin self,
        q: &'b I,
        fields: &[SearchType],
    ) -> Result<Option<Vec<&'pin Pin<'pin>>>, Error>
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
                .map(|p: &Vec<CachedPin<'pin>>| {
                    p.iter()
                        .filter(|cached_pin: &&CachedPin<'pin>| {
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
                        .collect::<Vec<&'pin Pin>>()
                })
                .unwrap_or_default()
        } else {
            let regex_queries = q
                .into_iter()
                .map(|s| {
                    let query = &s.as_ref().to_lowercase();
                    // Build a string for regex: "HAMID" => "H.*A.*M.*I.*D"
                    let mut fuzzy_string = String::with_capacity(query.len() * query.len() * 2);
                    fuzzy_string.push_str(
                        &query
                            .chars()
                            .map(|c| c.to_string())
                            .collect::<Vec<String>>()
                            .join(r".*"),
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
                    p.iter()
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
                .unwrap_or_default()
        };

        match results.len() {
            0 => Ok(None),
            _ => Ok(Some(results)),
        }
    }
}
