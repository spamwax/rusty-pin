use reqwest::IntoUrl;
use std::borrow::Cow;
use url_serde;

use chrono::prelude::*;
use url::Url;

use regex::Regex;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct Tag(pub String, pub usize);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Pin<'pin> {
    #[serde(with = "url_serde", rename = "href")]
    pub url: Url,
    #[serde(rename = "description")]
    pub title: Cow<'pin, str>,
    pub tags: Cow<'pin, str>,
    pub shared: Cow<'pin, str>,
    pub toread: Cow<'pin, str>,
    pub extended: Option<Cow<'pin, str>>,
    #[serde(default = "Utc::now")]
    pub time: DateTime<Utc>,
}

impl<'pin> Pin<'pin> {
    pub fn time(&self) -> DateTime<Utc> {
        self.time
    }

    pub fn contains(&self, q: &str) -> bool {
        self.title.to_lowercase().contains(q) || self.tags.to_lowercase().contains(q)
            || self.url.as_ref().contains(q)
            || (self.extended.is_some()
                && self.extended.as_ref().unwrap().to_lowercase().contains(q))
    }

    pub fn title_contains(&self, q: &str, re: Option<&Regex>) -> bool {
        if let Some(re) = re {
            re.is_match(&self.title)
        } else {
            self.title.to_lowercase().contains(q)
        }
    }

    pub fn tag_contains(&self, q: &str, re: Option<&Regex>) -> bool {
        if let Some(re) = re {
            re.is_match(&self.tags)
        } else {
            self.tags.to_lowercase().contains(q)
        }
    }

    pub fn url_contains(&self, q: &str, re: Option<&Regex>) -> bool {
        if let Some(re) = re {
            re.is_match(self.url.as_str())
        } else {
            self.url.as_str().to_lowercase().contains(q)
        }
    }

    pub fn extended_contains(&self, q: &str, re: Option<&Regex>) -> bool {
        self.extended.is_some() && if let Some(re) = re {
            re.is_match(self.extended.as_ref().unwrap())
        } else {
            self.extended.as_ref().unwrap().to_lowercase().contains(q)
        }
    }

    pub fn contains_fuzzy(&self, re: &Regex) -> bool {
        re.is_match(&self.title) || re.is_match(&self.tags) || re.is_match(self.url.as_ref())
            || (self.extended.is_some() && re.is_match(self.extended.as_ref().unwrap()))
    }
}

#[derive(Debug)]
pub struct PinBuilder<'pin> {
    pin: Pin<'pin>,
}

impl<'pin> PinBuilder<'pin> {
    pub fn new<T, S>(url: T, title: S) -> Self
    where
        T: IntoUrl,
        S: Into<Cow<'pin, str>>,
    {
        let pin = Pin {
            url: url.into_url().expect("Invalid url"),
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
    pub fn tags<S: Into<Cow<'pin, str>>>(mut self, t: S) -> Self {
        self.pin.tags = t.into();
        self
    }

    pub fn shared<S: Into<Cow<'pin, str>>>(mut self, f: S) -> Self {
        self.pin.shared = f.into();
        self
    }

    pub fn toread<S: Into<Cow<'pin, str>>>(mut self, f: S) -> Self {
        self.pin.toread = f.into();
        self
    }

    pub fn description<S: Into<Cow<'pin, str>>>(mut self, x: S) -> Self {
        self.pin.extended = Some(x.into());
        self
    }

    pub fn into_pin(self) -> Pin<'pin> {
        self.pin
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use env_logger;
    use std::env;

    use pinboard::mockito_helper::create_mockito_servers;

    #[test]
    fn test_builder() {
        let _ = env_logger::try_init();
        debug!("test_builder: starting");
        let p = PinBuilder::new(
            "https://githuуй.com/Здравствуйт?q=13#fragment",
            "title",
        ).tags("tag1 tag2")
            .into_pin();
        assert_eq!(p.title, "title");
        assert_eq!(
            p.url,
            Url::parse("https://githuуй.com/Здравствуйт?q=13#fragment").unwrap()
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
        ).tags("tag1 tag2")
            .into_pin();

        assert!(p.contains("·"));
        assert!(p.contains("· PlatformIO".to_lowercase().as_str()));
        assert!(p.contains("IoT".to_lowercase().as_str()));
        assert!(p.contains("tag".to_lowercase().as_str()));
        assert!(p.contains("tag1".to_lowercase().as_str()));
    }

    #[test]
    fn test_search_pins() {
        let _ = env_logger::try_init();
        debug!("test_search_pins: starting");

        let (_m1, _m2) = create_mockito_servers();

        let mut _home = env::home_dir().unwrap();
        _home.push(".cache");
        _home.push("mockito-rusty-pin");
        let cache_path = Some(_home);
        let p = ::pinboard::Pinboard::new(include_str!("api_token.txt"), cache_path)
            .map_err(|e| format!("{:?}", e));
        let mut pinboard = p.unwrap_or_else(|e| panic!("{:?}", e));

        pinboard.enable_tag_only_search(false);
        pinboard.enable_fuzzy_search(false);

        {
            // non-fuzzy search
            let pins = pinboard.search_items("rust").unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
        }

        {
            // fuzzy search
            pinboard.enable_fuzzy_search(true);
            pinboard.enable_tag_only_search(false);
            let pins = pinboard
                .search_items(r#"openstm"#)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
        }

        {
            let pins = pinboard
                .search_items("non-existence-pin")
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_none());
        }

        {
            // non-fuzzy search
            let pins = pinboard
                .search_items("failure - Cargo: packages for Rust")
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
            let pins = pins.unwrap();
            assert_eq!(pins.len(), 1);
            assert_eq!(pins[0].url.as_str(), "https://crates.io/crates/failure");
        }

        {
            // fuzzy search
            pinboard.enable_fuzzy_search(true);
            let pins = pinboard.search_items("failurecargopackage") // "failure cargo package"
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
            let pins = pins.unwrap();
            assert_eq!(pins.len(), 1);
            assert_eq!(pins[0].url.as_str(), "https://crates.io/crates/failure");
        }
    }
}
