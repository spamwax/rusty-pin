extern crate url;

use url::Url;

#[derive(Debug)]
pub struct Pin {
    url: Url,
    title: String,
    tags: Vec<String>,
    private: bool,
    read: bool,
    desc: Option<String>,
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
            url,
            title,
            tags,
            private,
            read,
            desc,
        }
    }

    pub fn contains(&self, q: &str) -> bool {
        self.url.as_ref().contains(q) || self.title.contains(q) ||
            self.tags.iter().any(|t| t.contains(q))
    }

    pub fn set_tags_str(&mut self, tags: &[&str]) -> () {
        self.tags = tags.iter().map(|s| s.to_string()).collect();
    }

    pub fn set_tags(&mut self, tags: Vec<String>) -> () {
        self.tags = tags;
    }
}

mod tests;
