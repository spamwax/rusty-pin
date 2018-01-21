#![feature(test)]

extern crate test;

extern crate chrono;
extern crate url;

extern crate regex;
extern crate rmp_serde as rmps;
#[macro_use]
extern crate serde_derive;

extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate url_serde;

pub mod pinboard;

pub use pinboard::{Pin, PinBuilder, Pinboard, Tag};

// TODO: Fix tests so we don't have to pass --test-threads=1. It seems issue is related to
// multithread access to cache files as some tests maybe deleting/updating while others reading.
// TODO: Improve performance! Maybe use some other encoding for saving cache files.
// TODO: Use buffer reading/writing when dealing with cache files.
// TODO: Use 'failure' crate for better error handling.
// TODO: Use threads to improve search speed?
#[cfg(test)]
mod tests {
    mod rmp_serde {
        use url::Url;
        use chrono::prelude::*;
        use std::fs::File;
        use std::fs;
        use std::io::prelude::*;
        use rmps::{Deserializer, Serializer};
        use serde::{Deserialize, Serialize};
        use serde_json;

        use pinboard::pin::{Pin, PinBuilder};
        //        use pinboard::cached_data::{CachedData, CachedPin};

        use test::Bencher;

        #[test]
        fn serialize_a_pin() {
            let mut pin = PinBuilder::new(
                "https://danielkeep.github.io/tlborm/book/README.html",
                "The Little Book of Rust Macros".to_string(),
            ).tags("Rust macros".to_string())
                .toread("yes")
                .shared("no")
                .description("WoW!!!".to_string())
                .into_pin();
            pin.time = Utc.ymd(2017, 5, 22).and_hms(17, 46, 54);

            let mut buf: Vec<u8> = Vec::new();
            pin.serialize(&mut Serializer::new(&mut buf)).unwrap();
            assert_eq!(133, buf.len());

            let mut fp = File::create("/tmp/test_rmp_serde.bin").unwrap();
            fp.write_all(buf.as_slice()).unwrap();
        }

        #[test]
        fn deserialize_a_pin() {
            serialize_a_pin();
            let fp = File::open("/tmp/test_rmp_serde.bin").unwrap();

            let mut de = Deserializer::from_read(fp);
            let pin: Pin = Deserialize::deserialize(&mut de).unwrap();

            assert_eq!(pin.title, "The Little Book of Rust Macros");
            assert_eq!(pin.time(), Utc.ymd(2017, 5, 22).and_hms(17, 46, 54));
            assert_eq!(pin.tags, "Rust macros");
            assert_eq!("yes", &pin.toread);
            assert_eq!("WoW!!!", &pin.extended.unwrap());
            assert_eq!(
                pin.url,
                Url::parse("https://danielkeep.github.io/tlborm/book/README.html").unwrap()
            );
            fs::remove_file("/tmp/test_rmp_serde.bin");
        }

        #[test]
        fn serialize_lots_of_pins() {
            let input = include_str!("../sample.json");
            let pins: Vec<Pin> = serde_json::from_str(input).unwrap();
            assert_eq!(612, pins.len());

            let mut buf: Vec<u8> = Vec::new();
            pins.serialize(&mut Serializer::new(&mut buf)).unwrap();
            assert_eq!(115671, buf.len());

            let mut fp = File::create("/tmp/test_rmp_serde-vec.bin").unwrap();
            fp.write_all(buf.as_slice()).unwrap();
        }

        #[test]
        fn deserialize_lots_of_pins() {
            serialize_lots_of_pins();
            let fp = File::open("tests/test_rmp_serde-vec.bin").unwrap();
            let mut de = Deserializer::from_read(fp);
            let pins: Vec<Pin> = Deserialize::deserialize(&mut de).unwrap();
            assert_eq!(pins.len(), 472);
            fs::remove_file("/tmp/test_rmp_serde-vec.bin");
        }

        #[bench]
        fn bench_rmp(b: &mut Bencher) {
            let bytes = include_bytes!("../tests/test_rmp_serde-vec.bin");
            b.iter(|| {
                let _pins: Vec<Pin> =
                    Deserialize::deserialize(&mut Deserializer::from_slice(bytes)).unwrap();
            })
        }

    } /* rmp_serde */

    mod json_serde {
        use super::*;
        use url::Url;
        use chrono::prelude::*;

        use pinboard::pin::{Pin, PinBuilder};
        use serde_json::{from_str, to_string};

        use test::Bencher;

        #[test]
        fn deserialize_a_pin() {
            let pin: Result<Pin, _> = from_str(include_str!("../tests/PIN1.json"));
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

            let pin: Result<Pin, _> = from_str(include_str!("../tests/PIN2.json"));
            assert!(pin.is_ok());
            let pin: Pin = pin.unwrap();
            // println!("{:?}", pin);
            assert_eq!(pin.title, "tbaggery - Effortless Ctags with Git");
            assert_eq!(pin.time(), Utc.ymd(2017, 10, 9).and_hms(7, 59, 36));
            assert_eq!(pin.tags, "git ctags vim");
            assert_eq!(
                pin.url,
                Url::parse("http://tbaggery.com/2011/08/08/effortless-ctags-with-git.html")
                    .unwrap()
            );
        }

        #[test]
        fn deserialize_two_pins() {
            let input = format!(
                "[{},{}]",
                include_str!("../tests/PIN1.json"),
                include_str!("../tests/PIN2.json")
            );
            let pins: Result<Vec<Pin>, _> = from_str(&input);
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
            let pins: Result<Vec<Pin>, _> = from_str(input);
            assert!(pins.is_ok());
            let pins = pins.unwrap();
            assert_eq!(612, pins.len());
        }

        #[bench]
        fn bench_json(b: &mut Bencher) {
            let input = include_str!("../sample.json");
            b.iter(|| {
                let _pins: Vec<Pin> = from_str(input).unwrap();
            });
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
            let s = to_string(&pin).unwrap();
            assert_eq!(
                r#"{"href":"https://danielkeep.github.io/tlborm/book/README.html",
"description":"The Little Book of Rust Macros","tags":"Rust macros","shared":"no"
,"toread":"no","extended":null,"time":"2017-05-22T17:46:54Z"}"#.replace("\n", ""),
                s
            );
        }
    }
}
