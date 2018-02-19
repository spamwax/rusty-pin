#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "bench", feature(test))]
#![cfg_attr(feature = "dev", plugin(clippy))]
#![cfg_attr(feature = "dev",
            warn(cast_possible_truncation, cast_possible_wrap, cast_precision_loss,
                 cast_sign_loss, mut_mut, non_ascii_literal, result_unwrap_used, shadow_reuse,
                 shadow_same, unicode_not_nfc, wrong_self_convention, wrong_pub_self_convention))]
#![cfg_attr(feature = "dev", allow(string_extend_chars))]

#[cfg(feature = "bench")]
extern crate test;

extern crate chrono;
extern crate url;

#[cfg(test)]
extern crate mockito;
extern crate regex;
extern crate rmp_serde as rmps;
#[macro_use]
extern crate serde_derive;

extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate url_serde;

#[macro_use]
extern crate failure;
#[macro_use]
extern crate failure_derive;

extern crate env_logger;
#[macro_use]
extern crate log;

pub mod pinboard;

pub use pinboard::{Pin, PinBuilder, Pinboard, Tag};

// TODO: Properly escape search queries that are used in regex for fuzzy option. <06-02-18, Hamid>
// TODO: Fix tests so we don't have to pass --test-threads=1. It seems issue is related to
// multithread access to cache files as some tests maybe deleting/updating while others reading.
// TODO: Improve performance! Maybe use some other encoding for saving cache files.
// TODO: Use 'failure' crate for better error handling.
// TODO: Use threads to improve search speed?
// TODO: Use Cow<'a, str> for user facing API? <19-02-18, Hamid>
#[cfg(test)]
mod tests {
    mod rmp_serde {
        use url::Url;
        use chrono::prelude::*;
        use std::fs::File;
        use std::{env, fs};
        use std::io::prelude::*;
        use rmps::{Deserializer, Serializer};
        use serde::{Deserialize, Serialize};
        use serde_json;
        use env_logger;

        use pinboard::pin::{Pin, PinBuilder};
        //        use pinboard::cached_data::{CachedData, CachedPin};

        #[cfg(feature = "bench")]
        use test::Bencher;

        #[test]
        fn serialize_a_pin() {
            let _ = env_logger::try_init();
            debug!("serialize_a_pin: starting");
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

            let mut dir = env::temp_dir();
            dir.push("test_rmp_serde.bin");

            let mut fp = File::create(dir).expect("Couldn't create temp file test_rmp_serde.bin");
            fp.write_all(buf.as_slice())
                .expect("Can't delete temp file");
        }

        #[test]
        fn deserialize_a_pin() {
            let _ = env_logger::try_init();
            debug!("deserialize_a_pin: starting");
            serialize_a_pin();

            let mut dir = env::temp_dir();
            dir.push("test_rmp_serde.bin");
            let fp = File::open(&dir).expect("Couldn't read temp file test_rmp_serde.bin");

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
            let _ = fs::remove_file(dir).expect("Can't delete temp test file");
        }

        #[test]
        fn serialize_lots_of_pins() {
            let _ = env_logger::try_init();
            debug!("serialize_lots_of_pins: starting");
            let input = include_str!("../sample.json");
            let pins: Vec<Pin> = serde_json::from_str(input).unwrap();
            assert_eq!(612, pins.len());

            let mut buf: Vec<u8> = Vec::new();
            pins.serialize(&mut Serializer::new(&mut buf)).unwrap();
            assert_eq!(115671, buf.len());

            let mut dir = env::temp_dir();
            dir.push("test_rmp_serde-vec.bin");
            let mut fp = File::create(dir).expect("Couldn't create temp file test_rmp_serde.bin");
            fp.write_all(buf.as_slice()).unwrap();
        }

        #[test]
        fn deserialize_lots_of_pins() {
            let _ = env_logger::try_init();
            debug!("deserialize_lots_of_pins: starting");
            serialize_lots_of_pins();

            let mut dir = env::temp_dir();
            dir.push("test_rmp_serde-vec.bin");

            let fp = File::open(&dir).expect("Couldn't create temp file test_rmp_serde.bin");
            let mut de = Deserializer::from_read(fp);
            let pins: Vec<Pin> = Deserialize::deserialize(&mut de).unwrap();
            assert_eq!(612, pins.len());
            let _ = fs::remove_file(dir).expect("Can't delete temp test file");
        }

        #[cfg(feature = "bench")]
        #[bench]
        fn bench_rmp(b: &mut Bencher) {
            let _ = env_logger::try_init();
            debug!("bench_rmp: starting");
            let bytes = include_bytes!("../tests/test_rmp_serde-vec.bin");
            b.iter(|| {
                let _pins: Vec<Pin> =
                    Deserialize::deserialize(&mut Deserializer::from_slice(bytes)).unwrap();
            })
        }

    } /* rmp_serde */

    mod json_serde {
        use url::Url;
        use chrono::prelude::*;
        use env_logger;

        use pinboard::pin::{Pin, PinBuilder};
        use serde_json::{from_str, to_string};

        #[cfg(feature = "bench")]
        use test::Bencher;

        #[test]
        fn deserialize_a_pin() {
            let _ = env_logger::try_init();
            debug!("deserialize_a_pin: starting");
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
            let _ = env_logger::try_init();
            debug!("deserialize_two_pins: starting");
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
            let _ = env_logger::try_init();
            debug!("deserialize_lots_pins: starting");
            let input = include_str!("../sample.json");
            let pins: Result<Vec<Pin>, _> = from_str(input);
            assert!(pins.is_ok());
            let pins = pins.unwrap();
            assert_eq!(612, pins.len());
        }

        #[cfg(feature = "bench")]
        #[bench]
        fn bench_json(b: &mut Bencher) {
            let _ = env_logger::try_init();
            debug!("bench_json: starting");
            let input = include_str!("../sample.json");
            b.iter(|| {
                let _pins: Vec<Pin> = from_str(input).unwrap();
            });
        }

        #[test]
        fn serialize_a_pin() {
            let _ = env_logger::try_init();
            debug!("serialize_a_pin: starting");
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
