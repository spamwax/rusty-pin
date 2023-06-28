// use reqwest::IntoUrl;
// #![allow(clippy::must_use_candidate)]
use std::borrow::Cow;

use chrono::prelude::*;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Pin<'pin> {
    #[serde(rename = "href")]
    pub url: Cow<'pin, str>,
    #[serde(rename = "description")]
    pub title: Cow<'pin, str>,
    pub tags: Cow<'pin, str>,
    pub shared: Cow<'pin, str>,
    pub toread: Cow<'pin, str>,
    pub extended: Option<Cow<'pin, str>>,
    #[serde(default = "Utc::now")]
    pub time: DateTime<Utc>,
}

use unicode_normalization::{is_nfkd_quick, IsNormalized};
impl<'pin> Pin<'pin> {
    #[allow(clippy::must_use_candidate)]
    pub fn time(&self) -> DateTime<Utc> {
        self.time
    }

    /// # Panics
    ///
    /// It pancis if the `q` is not a normalized unicode (nfk)
    #[allow(clippy::must_use_candidate)]
    pub fn contains(&self, q: &str) -> bool {
        assert!(is_nfkd_quick(q.chars()) == IsNormalized::Yes);
        self.title.to_lowercase().contains(q)
            || self.tags.to_lowercase().contains(q)
            || self.url.as_ref().contains(q)
            || if let Some(ref extended) = self.extended {
                extended.to_lowercase().contains(q)
            } else {
                false
            }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn title_contains(&self, q: &str, matcher: Option<&SkimMatcherV2>) -> bool {
        if let Some(matcher) = matcher {
            matcher.fuzzy_match(&self.title, q).is_some()
        } else {
            self.title.to_lowercase().contains(q)
        }
    }

    /// # Panics
    ///
    /// It pancis if the `q` is not a normalized unicode (nfk)
    #[must_use]
    pub fn tag_contains(&self, q: &str, matcher: Option<&SkimMatcherV2>) -> bool {
        assert!(is_nfkd_quick(q.chars()) == IsNormalized::Yes);
        if let Some(matcher) = matcher {
            matcher.fuzzy_match(&self.tags, q).is_some()
        } else {
            self.tags.to_lowercase().contains(q)
        }
    }

    /// # Panics
    ///
    /// It pancis if the `q` is not a normalized unicode (nfk)
    #[must_use]
    pub fn url_contains(&self, q: &str, matcher: Option<&SkimMatcherV2>) -> bool {
        assert!(is_nfkd_quick(q.chars()) == IsNormalized::Yes);
        if let Some(matcher) = matcher {
            matcher.fuzzy_match(&self.url, q).is_some()
        } else {
            self.url.to_lowercase().contains(q)
        }
    }

    /// # Panics
    ///
    /// It pancis if the `q` is not a normalized unicode (nfk)
    #[must_use]
    pub fn extended_contains(&self, q: &str, matcher: Option<&SkimMatcherV2>) -> bool {
        assert!(is_nfkd_quick(q.chars()) == IsNormalized::Yes);
        if let Some(ref extended) = self.extended {
            if let Some(matcher) = matcher {
                matcher.fuzzy_match(extended, q).is_some()
            } else {
                extended.to_lowercase().contains(q)
            }
        } else {
            false
        }
    }

    /// # Panics
    ///
    /// It pancis if the `q` is not a normalized unicode (nfk)
    pub fn contains_fuzzy(&self, q: &str, matcher: &SkimMatcherV2) -> bool {
        assert!(is_nfkd_quick(q.chars()) == IsNormalized::Yes);
        matcher.fuzzy_match(&self.tags, q).is_some()
            || matcher.fuzzy_match(&self.title, q).is_some()
            || matcher.fuzzy_match(self.url.as_ref(), q).is_some()
            || if let Some(ref extended) = self.extended {
                matcher.fuzzy_match(extended, q).is_some()
            } else {
                false
            }
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct PinBuilder<'pin> {
    pin: Pin<'pin>,
}

impl<'pin> PinBuilder<'pin> {
    pub fn new<S>(url: S, title: S) -> Self
    where
        S: Into<Cow<'pin, str>>,
    {
        let pin = Pin {
            url: url.into(),
            title: title.into(),
            time: Utc::now(),
            tags: Cow::from(""),
            shared: Cow::from(""),
            toread: Cow::from(""),
            extended: None,
        };
        PinBuilder { pin }
    }
}

impl<'pin> PinBuilder<'pin> {
    #[must_use]
    pub fn tags<S: Into<Cow<'pin, str>>>(mut self, t: S) -> Self {
        self.pin.tags = t.into();
        self
    }

    #[must_use]
    pub fn shared<S: Into<Cow<'pin, str>>>(mut self, f: S) -> Self {
        self.pin.shared = f.into();
        self
    }

    #[must_use]
    pub fn toread<S: Into<Cow<'pin, str>>>(mut self, f: S) -> Self {
        self.pin.toread = f.into();
        self
    }

    #[must_use]
    pub fn description<S: Into<Cow<'pin, str>>>(mut self, x: S) -> Self {
        self.pin.extended = Some(x.into());
        self
    }

    #[must_use]
    pub fn into_pin(self) -> Pin<'pin> {
        self.pin
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use env_logger;

    use crate::pinboard::mockito_helper::create_mockito_servers;

    #[test]
    fn pin_builder_test() {
        let _ = env_logger::try_init();
        debug!("test_builder: starting");
        let p = PinBuilder::new("https://githuуй.com/Здравствуйт?q=13#fragment", "title")
            .tags("tag1 tag2")
            .into_pin();
        assert_eq!(p.title, "title");
        assert_eq!(
            &p.url,
            "https://githuуй.com/Здравствуйт?q=13#fragment" // Url::parse("https://githuуй.com/Здравствуйт?q=13#fragment")
                                                                         //     .expect("impossible")
                                                                         //     .as_str()
        );
        assert_eq!(p.tags, "tag1 tag2");
    }

    #[test]
    fn test_pin_contain() {
        let _ = env_logger::try_init();
        debug!("test_pin_contain: starting");
        let p = PinBuilder::new(
            "http://правительство.рф",
            "An open source ecosystem for IoT development · PlatformIO",
        )
        .tags("tag1 tag2")
        .into_pin();

        assert!(p.contains("·"));
        assert!(p.contains("· PlatformIO".to_lowercase().as_str()));
        assert!(p.contains("IoT".to_lowercase().as_str()));
        assert!(p.contains("tag".to_lowercase().as_str()));
        assert!(p.contains("tag1".to_lowercase().as_str()));
    }

    #[test]
    fn test_tag_search_with_diff_case() {
        let _ = env_logger::try_init();
        debug!("test_search_pins: starting");

        let (_m1, _m2) = create_mockito_servers();

        let mut myhome = dirs::home_dir().unwrap();
        myhome.push(".cache");
        myhome.push("mockito-rusty-pin");
        let cache_path = Some(myhome);
        let p = crate::pinboard::Pinboard::new(include_str!("api_token.txt"), cache_path)
            .map_err(|e| format!("{e:?}"));
        let mut pinboard = p.unwrap_or_else(|e| panic!("{e:?}")).pinboard;

        pinboard.enable_tag_only_search(true);
        pinboard.enable_fuzzy_search(false);
        let c1;
        let c2;
        {
            let pins = pinboard
                .search_items("Rust")
                .unwrap_or_else(|e| panic!("{e:?}"));
            assert!(pins.is_some());
            c1 = pins.unwrap().len();
        }
        {
            let pins = pinboard
                .search_items("rust")
                .unwrap_or_else(|e| panic!("{e:?}"));
            assert!(pins.is_some());
            c2 = pins.unwrap().len();
        }
        assert_eq!(10, c1);
        assert_eq!(c1, c2);

        pinboard.enable_tag_only_search(false);
        let tags1 = pinboard.search_list_of_tags("gi");
        let tags2 = pinboard.search_list_of_tags("Gi");
        assert!(tags1.is_ok());
        assert!(tags2.is_ok());
        assert_eq!(1, tags1.unwrap().unwrap().len());
        assert_eq!(1, tags2.unwrap().unwrap().len());
    }

    #[test]
    fn test_search_pins() {
        let _ = env_logger::try_init();
        debug!("test_search_pins: starting");

        let (_m1, _m2) = create_mockito_servers();

        let mut myhome = dirs::home_dir().unwrap();
        myhome.push(".cache");
        myhome.push("mockito-rusty-pin");
        let cache_path = Some(myhome);
        let p = crate::pinboard::Pinboard::new(include_str!("api_token.txt"), cache_path)
            .map_err(|e| format!("{e:?}"));
        let mut pinboard = p.unwrap_or_else(|e| panic!("{e:?}")).pinboard;

        pinboard.enable_tag_only_search(false);
        pinboard.enable_fuzzy_search(false);

        {
            // non-fuzzy search
            let pins = pinboard
                .search_items("rust")
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_some());
        }

        {
            // fuzzy search
            pinboard.enable_fuzzy_search(true);
            pinboard.enable_tag_only_search(false);
            let pins = pinboard
                .search_items(r#"openstm"#)
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_some());
        }

        {
            let pins = pinboard
                .search_items("non-existence-pin")
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_none());
        }

        {
            // non-fuzzy search
            let pins = pinboard
                .search_items("failure - Cargo: packages for Rust")
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_some());
            let pins = pins.unwrap();
            assert_eq!(pins.len(), 1);
            assert_eq!(&pins[0].url, "https://crates.io/crates/failure");
        }

        {
            // fuzzy search
            pinboard.enable_fuzzy_search(true);
            let pins = pinboard
                .search_items("failurecargopackage") // "failure cargo package"
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_some());
            let pins = pins.unwrap();
            assert_eq!(pins.len(), 1);
            assert_eq!(&pins[0].url, "https://crates.io/crates/failure");
        }
    }
}
