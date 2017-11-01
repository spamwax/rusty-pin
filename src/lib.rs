extern crate chrono;
extern crate url;

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;
extern crate url_serde;
extern crate reqwest;

use url::Url;
use chrono::prelude::*;

mod pinboard;

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
    time: DateTime<Utc>,
    #[serde(skip)]
    meta: Option<String>,
    #[serde(skip)]
    hash: Option<String>,
    #[serde(skip)]
    tag_list: Vec<String>,
}

impl Pin {
    // TODO: Add a 'builder' to construcet a new Pin //
    pub fn new(
        url: Url,
        title: String,
        tags: Vec<String>,
        private: bool,
        read: bool,
        desc: Option<String>,
    ) -> Pin {
        let shared;
        let toread;
        if private {
            shared = "no";
        } else {
            shared = "yes";
        }
        if read {
            toread = "yes";
        } else {
            toread = "no";
        }
        Pin {
            url,
            title,
            tags: String::new(),
            shared: shared.to_string(),
            toread: toread.to_string(),
            extended: desc,
            time: Utc::now(),
            meta: None,
            hash: None,
            tag_list: tags,
        }
    }

    pub fn contains(&self, q: &str) -> bool {
        self.title.contains(q) || self.url.as_ref().contains(q) || self.tags.contains(q)
    }

    pub fn set_tags_str(&mut self, tags: &[&str]) -> () {
        self.tag_list = tags.iter().map(|s| s.to_string()).collect();
    }

    pub fn set_tags(&mut self, tags: Vec<String>) -> () {
        self.tag_list = tags;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_pin(url: &str, title: &str) -> Pin {
        let url = Url::parse(url).unwrap();
        Pin::new(url, title.to_string(), vec![], true, false, None)
    }

    #[test]
    fn set_tags_from_vec() {
        let mut p =
            create_pin("https://githuуй.com/Здравствуйт?q=13#fragment", "title");

        let tags = vec!["tag1", "tag2"];
        p.set_tags_str(&tags);
        assert_eq!(p.tag_list, tags);
    }

    #[test]
    fn set_tags_from_vec_string() {
        let mut p =
            create_pin("https://githuуй.com/Здравствуйт?q=13#fragment", "title");

        let tags = vec![String::from("tag3"), "tag4".to_string()];
        p.set_tags_str(
            tags.iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>()
                .as_slice(),
        );
        assert_eq!(p.tag_list, tags);
    }

    #[test]
    fn set_tags_from_vec_clone() {
        let mut p =
            create_pin("https://githuуй.com/Здравствуйт?q=13#fragment", "title");

        let tags = vec![String::from("tag5"), "tag6".to_string()];
        p.set_tags(tags.clone());
        assert_eq!(p.tag_list, tags);
    }

    #[test]
    fn deserialize_a_pin() {
        let pin: Result<Pin, _> = serde_json::from_str(include_str!("../tests/PIN1.json"));
        assert!(pin.is_ok());
        let pin: Pin = pin.unwrap();
        // println!("{:?}", pin);
        assert_eq!(pin.title, "The Little Book of Rust Macros");
        assert_eq!(pin.time, Utc.ymd(2017, 5, 22).and_hms(17, 46, 54));
        assert_eq!(pin.tags, "Rust macros");
        assert_eq!(
            pin.url,
            Url::parse("https://danielkeep.github.io/tlborm/book/README.html").unwrap()
        );

        let pin: Result<Pin, _> = serde_json::from_str(include_str!("../tests/PIN2.json"));
        assert!(pin.is_ok());
        let pin: Pin = pin.unwrap();
        // println!("{:?}", pin);
        assert_eq!(pin.title, "tbaggery - Effortless Ctags with Git");
        assert_eq!(pin.time, Utc.ymd(2017, 10, 9).and_hms(7, 59, 36));
        assert_eq!(pin.tags, "git ctags vim");
        assert_eq!(
            pin.url,
            Url::parse("http://tbaggery.com/2011/08/08/effortless-ctags-with-git.html").unwrap()
        );
    }

    #[test]
    fn deserialize_two_pins() {
        let input = format!(
            "[{},{}]",
            include_str!("../tests/PIN1.json"),
            include_str!("../tests/PIN2.json")
        );
        let pins: Result<Vec<Pin>, _> = serde_json::from_str(&input);
        if let Err(e) = pins {
            // println!("{:?}", e);
            return;
        }
        assert!(pins.is_ok());
        let pins = pins.unwrap();
        assert_eq!(pins.len(), 2);
        // println!("{:?}", pins);
    }

    #[test]
    fn deserialize_lots_pins() {
        let input = include_str!("../sample.json");
        let pins: Result<Vec<Pin>, _> = serde_json::from_str(input);
        assert!(pins.is_ok());
        let pins = pins.unwrap();
        assert_eq!(pins.len(), 472);
    }

    #[test]
    fn serialize_a_pin() {
        let mut pin = create_pin(
            "https://danielkeep.github.io/tlborm/book/README.html",
            "The Little Book of Rust Macros",
        );
        pin.toread = "no".to_string();
        pin.shared = "no".to_string();
        pin.tags = "Rust macros".to_string();
        pin.time = Utc.ymd(2017, 5, 22).and_hms(17, 46, 54);
        let s = serde_json::to_string(&pin).unwrap();
        assert_eq!(
            r#"{"href":"https://danielkeep.github.io/tlborm/book/README.html",
"description":"The Little Book of Rust Macros","tags":"Rust macros","shared":"no"
,"toread":"no","time":"2017-05-22T17:46:54Z"}"#.replace("\n", ""),
            s
        );
    }
}
