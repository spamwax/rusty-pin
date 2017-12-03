extern crate chrono;
extern crate url;

#[macro_use]
extern crate serde_derive;
extern crate regex;


extern crate serde;
extern crate serde_json;
extern crate url_serde;
extern crate reqwest;

pub mod pinboard;

#[cfg(test)]
mod tests {
    mod serde_json {
        use super::*;
        use url::Url;
        use chrono::prelude::*;

        use pinboard::{Pin, PinBuilder};
        use serde_json::{to_string, from_str};

        #[test]
        fn deserialize_a_pin() {
            let pin: Result<Pin, _> = serde_json::from_str(include_str!("../tests/PIN1.json"));
            assert!(pin.is_ok());
            let pin: Pin = pin.unwrap();
            // println!("{:?}", pin);
            assert_eq!(pin.title, "The Little Book of Rust Macros");
            assert_eq!(pin.time(), Utc.ymd(2017, 5, 22).and_hms(17, 46, 54));
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
            assert_eq!(pin.time(), Utc.ymd(2017, 10, 9).and_hms(7, 59, 36));
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
                println!("{:?}", e);
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
            let mut pin = PinBuilder::new(
                "https://danielkeep.github.io/tlborm/book/README.html",
                "The Little Book of Rust Macros".to_string(),
            ).tags("Rust macros".to_string())
                .toread("no")
                .shared("no")
                .into_pin();
            pin.time = Utc.ymd(2017, 5, 22).and_hms(17, 46, 54);
            let s = serde_json::to_string(&pin).unwrap();
            assert_eq!(
                r#"{"href":"https://danielkeep.github.io/tlborm/book/README.html",
"description":"The Little Book of Rust Macros","tags":"Rust macros","shared":"no"
,"toread":"no","time":"2017-05-22T17:46:54Z"}"#
                    .replace("\n", ""),
                s
            );
        }
    }
}
