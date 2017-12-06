use url_serde;
use reqwest::IntoUrl;

use chrono::prelude::*;
use url::Url;

use regex::Regex;


#[derive(Serialize, Deserialize, Debug)]
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
    #[serde(skip)]
    pub tag_list: Vec<String>,
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

        self.title.to_lowercase().contains(q) || self.tags.to_lowercase().contains(q) ||
            self.url.as_ref().contains(q) ||
            (self.extended.is_some() && self.extended.as_ref().unwrap().to_lowercase().contains(q))
    }

    pub fn contains_fuzzy(&self, re: &Regex) -> bool {
        re.captures(&self.title).is_some() || re.captures(&self.tags).is_some() ||
            re.captures(&self.url.as_ref()).is_some() ||
            (self.extended.is_some() && re.captures(self.extended.as_ref().unwrap()).is_some())
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


