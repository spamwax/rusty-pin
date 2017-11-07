#![allow(dead_code)]
use url_serde;
use reqwest::IntoUrl;

use chrono::prelude::*;
use url::Url;

mod api;

#[derive(Debug)]
pub struct Pinboard {
    auth_token: String,
}

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
    pub fn new(auth_token: String) -> Self {
        Pinboard { auth_token }
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
        assert_eq!(p.tag_list.len(), 2);
        assert_eq!(p.tags, "tag1 tag2".to_string());
        assert_eq!(p.tag_list, vec!["tag1", "tag2"]);
    }
}
