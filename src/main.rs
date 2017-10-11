#![feature(core_intrinsics)]
extern crate url;

use url::Url;

#[derive(Debug)]
struct Pin {
    url: Url,
    title: String,
    tags: Vec<String>,
    private: bool,
    read: bool,
    desc: Option<String>,
}

impl Pin {
    fn new(url: Url, title: String, tags: Vec<String>,
           private: bool,
           read: bool,
           desc: Option<String>) -> Pin {
        Pin { url, title, tags, private, read, desc }
    }

    fn set_tags(&mut self, tags: &[&str]) -> () {
        self.tags = tags.iter().map(|s| s.to_string()).collect();
    }
}

fn main() {
    println!("Hello, world!");
}

fn print_type_of<T>(_: &T) {
    println!("{}", unsafe { std::intrinsics::type_name::<T>() });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_tags() {
        let url = Url::parse( "https://githubуй.com/Здравствуйтеу" ).unwrap();
        let mut p = Pin::new(url, "title".to_string(), vec![], true, false, None);

        let tags = vec!["tag1", "tag2"];
        p.set_tags(&tags);
        assert_eq!(p.tags, tags);

        let tags = vec![String::from("tag3"), "tag4".to_string()];
        p.set_tags(tags
            .iter()
            .map(|s| s.as_str()).collect::<Vec<&str>>()
            .as_slice());
        assert_eq!(p.tags, tags);
    }
}