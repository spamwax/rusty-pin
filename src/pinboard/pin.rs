use url_serde;
use reqwest::IntoUrl;

use chrono::prelude::*;
use url::Url;

use regex::Regex;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Tag(pub String, pub usize);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Pin {
    #[serde(with = "url_serde", rename = "href")]
    pub url: Url,
    #[serde(rename = "description")]
    pub title: String,
    pub tags: String,
    pub shared: String,
    pub toread: String,
    // #[serde(skip_serializing_if = "Option::is_none")]
    pub extended: Option<String>,
    #[serde(default = "Utc::now")]
    pub time: DateTime<Utc>,
    #[serde(skip)]
    meta: Option<String>,
    #[serde(skip)]
    hash: Option<String>,
}

impl Pin {
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
            re.captures(&self.title).is_some()
        } else {
            self.title.to_lowercase().contains(q)
        }
    }

    pub fn tag_contains(&self, q: &str, re: Option<&Regex>) -> bool {
        if let Some(re) = re {
            re.captures(&self.tags).is_some()
        } else {
            self.tags.to_lowercase().contains(q)
        }
    }

    pub fn url_contains(&self, q: &str, re: Option<&Regex>) -> bool {
        if let Some(re) = re {
            re.captures(self.url.as_str()).is_some()
        } else {
            self.url.as_str().to_lowercase().contains(q)
        }
    }

    pub fn extended_contains(&self, q: &str, re: Option<&Regex>) -> bool {
        self.extended.is_some() && if let Some(re) = re {
            re.captures(self.extended.as_ref().unwrap()).is_some()
        } else {
            self.extended.as_ref().unwrap().to_lowercase().contains(q)
        }
    }

    pub fn contains_fuzzy(&self, re: &Regex) -> bool {
        re.captures(&self.title).is_some() || re.captures(&self.tags).is_some()
            || (self.extended.is_some() && re.captures(self.extended.as_ref().unwrap()).is_some())
            || re.captures(self.url.as_ref()).is_some()
    }
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
        };
        PinBuilder { pin }
    }
}

impl PinBuilder {
    pub fn tags(mut self, t: String) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

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
        // assert_eq!(p.tag_list.len(), 2);
        assert_eq!(p.tags, "tag1 tag2".to_string());
        // assert_eq!(p.tag_list, vec!["tag1", "tag2"]);
    }

    #[test]
    fn test_pin_contain() {
        let p = PinBuilder::new(
            "http://правительство.рф",
            "An open source ecosystem for IoT development · PlatformIO".to_string(),
        ).tags("tag1 tag2".to_string())
            .into_pin();

        assert!(p.contains("·"));
        assert!(p.contains("· PlatformIO".to_lowercase().as_str()));
        assert!(p.contains("IoT".to_lowercase().as_str()));
        assert!(p.contains("tag".to_lowercase().as_str()));
        assert!(p.contains("tag1".to_lowercase().as_str()));
    }

    #[test]
    fn test_search_pins() {
        let p: Option<String> = None;
        let mut pinboard = ::pinboard::Pinboard::new(include_str!("auth_token.txt"), p)
            .expect("Unable to initiate Pinboard");
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
                .search_items("solvingbootp")
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
