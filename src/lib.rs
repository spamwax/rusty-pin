#![cfg_attr(feature = "bench", feature(test))]
// #![cfg_attr(feature = "dev", feature(plugin))]
// #[allow(clippy::string_extend_chars)]

#[cfg(feature = "bench")]
extern crate test;

extern crate chrono;
extern crate url;

#[cfg(test)]
extern crate mockito;
#[cfg(test)]
extern crate tempfile;

// extern crate regex;
extern crate rmp_serde as rmps;
#[macro_use]
extern crate serde_derive;

extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_json;

extern crate dirs;

extern crate env_logger;
#[macro_use]
extern crate log;

extern crate unicode_normalization;
// use unicode_normalization::{is_nfc, is_nfd, is_nfkc, is_nfkd};

pub mod pinboard;

pub use crate::pinboard::{CacheState, Pin, PinBuilder, Pinboard, PinboardResult, Tag};

// TODO: Use github actions for CI integration tests.
// TODO: make get_api_response return reqwest::Response so we can use serde_json::from_read
// TODO: Properly escape search queries that are used in regex for fuzzy option. <06-02-18, Hamid>
//       Some special chars to escape: (  ) | ? * + [  ]
// TODO: Fix tests so we don't have to pass --test-threads=1. It seems issue is related to
//       multithread access to cache files as some tests maybe
//       deleting/updating while others reading.
// TODO: Add proper rust formatted documentaiton
// TODO: make all tests use tempfile for the cache folder?
// TODO: Use RefCell or Cell to have inner mutability //
// TODO: What happens if no bookmark or no tags are stored in user's account.
// TODO: Find a way to reliably cause network errors when using Mockito. For example, make BASE_URL
//       a none const value so it can be set by test functions. That way we can set a bad BASE_URL to
//       cause network issues. Or, find a crate that does this type of error mocking! <08-07-22>

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    mod postcard_serde {
        use crate::pinboard::pin::Pin;
        use postcard::{from_bytes, to_allocvec};
        use std::fs::File;
        use std::io::prelude::*;
        use std::io::{BufReader, BufWriter};
        use std::{env, fs};
        #[cfg(feature = "bench")]
        use test::Bencher;

        #[test]
        fn serialize_lots_of_pins() {
            let _ = env_logger::try_init();
            debug!("serialize_lots_of_pins: starting");
            let input = include_str!("../sample.json");
            let pins: Vec<Pin> = serde_json::from_str(input).expect("Couldn't read sample.json");
            assert_eq!(612, pins.len());

            let buf: Vec<u8> = to_allocvec(&pins).expect("Couldn't serialize lots open");
            assert_eq!(114_293, buf.len());

            let mut dir = env::temp_dir();
            dir.push("test_postcard_serde-vec.bin");
            let fp =
                File::create(dir).expect("Couldn't create temp file test_postcard_serde-vec.bin");
            let mut writer = BufWriter::with_capacity(128_000, fp);
            writer
                .write_all(buf.as_slice())
                .expect("Can't write to test_rmp_postcard-vec.bin");
        }

        #[test]
        fn deserialize_lots_of_pins() {
            let _ = env_logger::try_init();
            debug!("deserialize_lots_of_pins: starting");
            serialize_lots_of_pins();

            let mut dir = env::temp_dir();
            dir.push("test_postcard_serde-vec.bin");

            let fp = File::open(&dir).expect("Couldn't open temp file test_postcard_serde.bin");
            let mut reader = BufReader::with_capacity(128_000, fp);

            let mut buf: Vec<u8> = Vec::with_capacity(128_000);
            let n = reader
                .read_to_end(&mut buf)
                .expect("Couldn't read deserialized data!");
            assert_eq!(114_293, n);

            let pins: Vec<Pin> = from_bytes(buf.as_slice()).unwrap();
            assert_eq!(612, pins.len());

            fs::remove_file(dir).expect("Can't delete temp test file");
        }

        #[cfg(feature = "bench")]
        #[allow(soft_unstable)]
        #[bench]
        fn bench_postcard(b: &mut Bencher) {
            let _ = env_logger::try_init();
            debug!("bench_postcard: starting");
            let bytes = include_bytes!("../tests/test_postcard_serde-vec.bin");
            let mut _pins: Vec<Pin> = Vec::with_capacity(1024);
            b.iter(|| {
                _pins = from_bytes(bytes).expect("Couldn't deserialize lots of pins");
            })
        }
    }

    mod rmp_serde {
        use crate::rmps::{Deserializer, Serializer};
        use chrono::prelude::*;
        use serde::{Deserialize, Serialize};
        use std::fs::File;
        use std::io::prelude::*;
        use std::io::{BufReader, BufWriter};
        use std::{env, fs};
        use url::Url;

        use crate::pinboard::pin::{Pin, PinBuilder};

        #[cfg(feature = "bench")]
        use test::Bencher;

        #[test]
        fn serialize_a_pin() {
            let _ = env_logger::try_init();
            debug!("serialize_a_pin: starting");
            let mut pin = PinBuilder::new(
                "https://danielkeep.github.io/tlborm/book/README.html",
                "The Little Book of Rust Macros",
            )
            .tags("Rust macros")
            .toread("yes")
            .shared("no")
            .description("WoW!!!")
            .into_pin();
            pin.time = Utc.with_ymd_and_hms(2017, 5, 22, 17, 46, 54).unwrap();

            let mut buf: Vec<u8> = Vec::new();
            pin.serialize(&mut Serializer::new(&mut buf))
                .expect("Couldn't serialize");
            assert_eq!(133, buf.len());

            let mut dir = env::temp_dir();
            dir.push("test_rmp_serde.bin");

            let fp = File::create(dir).expect("Couldn't create temp file test_rmp_serde.bin");
            let mut writer = BufWriter::with_capacity(256, fp);
            writer
                .write_all(buf.as_slice())
                .expect("Can't write to test_rmp_serde.bin");
        }

        #[test]
        fn deserialize_a_pin() {
            let _ = env_logger::try_init();
            debug!("deserialize_a_pin: starting");
            serialize_a_pin();

            let mut dir = env::temp_dir();
            dir.push("test_rmp_serde.bin");
            let fp = File::open(&dir).expect("Couldn't open temp file test_rmp_serde.bin");
            let reader = BufReader::with_capacity(256, fp);

            let mut de = Deserializer::new(reader);
            let pin: Pin =
                Deserialize::deserialize(&mut de).expect("Couldn't deserialize into pin.");

            assert_eq!(pin.title, "The Little Book of Rust Macros");
            assert_eq!(
                pin.time(),
                Utc.with_ymd_and_hms(2017, 5, 22, 17, 46, 54).unwrap()
            );
            assert_eq!(pin.tags, "Rust macros");
            assert_eq!("yes", &pin.toread);
            assert_eq!("WoW!!!", &pin.extended.expect("pin.extended can't be None"));
            assert_eq!(
                &pin.url,
                Url::parse("https://danielkeep.github.io/tlborm/book/README.html")
                    .expect("impossible")
                    .as_str()
            );
            fs::remove_file(dir).expect("Can't delete temp test file");
        }

        #[test]
        fn serialize_lots_of_pins() {
            let _ = env_logger::try_init();
            debug!("serialize_lots_of_pins: starting");
            let input = include_str!("../sample.json");
            let pins: Vec<Pin> = serde_json::from_str(input).expect("Couldn't read sample.json");
            assert_eq!(612, pins.len());

            let mut buf: Vec<u8> = Vec::new();
            pins.serialize(&mut Serializer::new(&mut buf))
                .expect("Couldn't serialize lots of pins");
            assert_eq!(115_671, buf.len());

            let mut dir = env::temp_dir();
            dir.push("test_rmp_serde-vec.bin");
            let fp = File::create(dir).expect("Couldn't create temp file test_rmp_serde-vec.bin");
            let mut writer = BufWriter::with_capacity(128_000, fp);
            writer
                .write_all(buf.as_slice())
                .expect("Can't write to test_rmp_serde-vec.bin");
        }

        #[test]
        fn deserialize_lots_of_pins() {
            let _ = env_logger::try_init();
            debug!("deserialize_lots_of_pins: starting");
            serialize_lots_of_pins();

            let mut dir = env::temp_dir();
            dir.push("test_rmp_serde-vec.bin");

            let fp = File::open(&dir).expect("Couldn't open temp file test_rmp_serde.bin");
            let reader = BufReader::with_capacity(128_000, fp);
            let mut de = Deserializer::new(reader);
            let pins: Vec<Pin> =
                Deserialize::deserialize(&mut de).expect("Couldn't deserialize into Vec<Pin>.");
            assert_eq!(612, pins.len());
            fs::remove_file(dir).expect("Can't delete temp test file");
        }

        #[cfg(feature = "bench")]
        #[bench]
        fn bench_rmp(b: &mut Bencher) {
            let _ = env_logger::try_init();
            debug!("bench_rmp: starting");
            let bytes = include_bytes!("../tests/test_rmp_serde-vec.bin");
            b.iter(|| {
                let _pins: Vec<Pin> =
                    Deserialize::deserialize(&mut Deserializer::from_read_ref(bytes))
                        .expect("Couldn't deserialize lots of pins");
            })
        }
    } /* rmp_serde */

    mod json_serde {
        use chrono::prelude::*;
        use url::Url;

        use crate::pinboard::pin::{Pin, PinBuilder};
        use serde_json::{from_str, to_string};

        #[cfg(feature = "bench")]
        use test::Bencher;

        #[test]
        fn deserialize_a_pin() {
            let _ = env_logger::try_init();
            debug!("deserialize_a_pin: starting");
            let pin: Result<Pin, _> = from_str(include_str!("../tests/PIN1.json"));
            assert!(pin.is_ok());
            let pin: Pin = pin.expect("impossible!");

            assert_eq!(pin.title, "The Little Book of Rust Macros");
            assert_eq!(
                pin.time(),
                Utc.with_ymd_and_hms(2017, 5, 22, 17, 46, 54).unwrap()
            );
            assert_eq!(pin.tags, "Rust macros");
            assert_eq!(
                &pin.url,
                Url::parse("https://danielkeep.github.io/tlborm/book/README.html")
                    .expect("impossible!")
                    .as_str()
            );

            let pin: Result<Pin, _> = from_str(include_str!("../tests/PIN2.json"));
            assert!(pin.is_ok());
            let pin: Pin = pin.expect("impossible");
            assert_eq!(pin.title, "tbaggery - Effortless Ctags with Git");
            assert_eq!(
                pin.time(),
                Utc.with_ymd_and_hms(2017, 10, 9, 7, 59, 36).unwrap()
            );
            assert_eq!(pin.tags, "git ctags vim");
            assert_eq!(
                &pin.url,
                Url::parse("http://tbaggery.com/2011/08/08/effortless-ctags-with-git.html")
                    .expect("impossible")
                    .as_str()
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
                println!("{e:?}");
                return;
            }
            assert!(pins.is_ok());
            let pins = pins.expect("impossible");
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
            let pins = pins.expect("impossible");
            assert_eq!(612, pins.len());
        }

        #[cfg(feature = "bench")]
        #[bench]
        fn bench_json(b: &mut Bencher) {
            let _ = env_logger::try_init();
            debug!("bench_json: starting");
            let input = include_str!("../sample.json");
            b.iter(|| {
                let _pins: Vec<Pin> = from_str(input).expect("Couldn't deserialize");
            });
        }

        #[test]
        fn serialize_a_pin() {
            let _ = env_logger::try_init();
            debug!("serialize_a_pin: starting");
            let mut pin = PinBuilder::new(
                "https://danielkeep.github.io/tlborm/book/README.html",
                "The Little Book of Rust Macros",
            )
            .tags("Rust macros")
            .toread("no")
            .shared("no")
            .into_pin();
            pin.time = Utc.with_ymd_and_hms(2017, 5, 22, 17, 46, 54).unwrap();
            let s = to_string(&pin).expect("Couldn't serialize");
            assert_eq!(
                r#"{"href":"https://danielkeep.github.io/tlborm/book/README.html",
"description":"The Little Book of Rust Macros","tags":"Rust macros","shared":"no"
,"toread":"no","extended":null,"time":"2017-05-22T17:46:54Z"}"#
                    .replace('\n', ""),
                s
            );
        }
    }

    pub(super) fn rand_temp_path() -> PathBuf {
        tempfile::Builder::new()
            .prefix("rusty_pin_test_")
            .rand_bytes(5)
            .tempdir()
            .expect("couldn't create tempdir")
            .into_path()
    }
}
