extern crate url;
extern crate chrono;

use url::Url;
use chrono::prelude::*;

#[derive(Debug)]
pub struct Pin {
    pub href: Url,
    pub description: String,
    pub tags: Vec<String>,
    pub private: bool,
    pub read: bool,
    pub extended: Option<String>,
    time: DateTime<Utc>,
    meta: Option<String>,
    hash: Option<String>
}

impl Pin {
    pub fn new(
        url: Url,
        title: String,
        tags: Vec<String>,
        private: bool,
        read: bool,
        desc: Option<String>,
    ) -> Pin {
        Pin {
            href: url,
            description: title,
            tags,
            private,
            read,
            extended: desc,
            time: Utc::now(),
            meta: None,
            hash: None,
        }
    }

    pub fn contains(&self, q: &str) -> bool {
        self.href.as_ref().contains(q) || self.description.contains(q) ||
            self.tags.iter().any(|t| t.contains(q))
    }

    pub fn set_tags_str(&mut self, tags: &[&str]) -> () {
        self.tags = tags.iter().map(|s| s.to_string()).collect();
    }

    pub fn set_tags(&mut self, tags: Vec<String>) -> () {
        self.tags = tags;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_tags() {
        let url = Url::parse("https://githuуй.com/Здравствуйт?q=13#fragment").unwrap();
        let mut p = Pin::new(url, "title".to_string(), vec![], true, false, None);

        let tags = vec!["tag1", "tag2"];
        p.set_tags_str(&tags);
        assert_eq!(p.tags, tags);

        let tags = vec![String::from("tag3"), "tag4".to_string()];
        p.set_tags_str(
            tags.iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>()
                .as_slice(),
        );
        assert_eq!(p.tags, tags);

        let tags = vec![String::from("tag5"), "tag6".to_string()];
        p.set_tags(tags.clone());
        assert_eq!(p.tags, tags);
    }
}
