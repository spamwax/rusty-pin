#![allow(clippy::unicode_not_nfc)]
#[allow(clippy::wildcard_imports)]
use super::*;
use env_logger;
use std::fs;
use std::io::Write;
use std::io::{BufReader, BufWriter};

use crate::rmps;
use serde::Serialize;

use self::tag::Tag;
use super::pin::Pin;

const TAGS_CACHE_FN: &str = "tags.cache";
const PINS_CACHE_FN: &str = "pins.cache";

const FILE_BUF_SIZE: usize = 4 * 1024 * 1024;
const CACHE_BUF_SIZE: usize = 1024;

#[derive(Debug)]
pub struct CachedData<'pin> {
    pub pins: Option<Vec<CachedPin<'pin>>>,
    pub tags: Option<Vec<CachedTag>>,
    pub cache_dir: PathBuf,
    pub tags_cache_file: PathBuf,
    pub pins_cache_file: PathBuf,
    cache_files_valid: bool,
}

// TODO: Add a url_lowered field to CachedPin so we don't have to call
//       .to_lowercase() in pinboard.find_url() every time //
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct CachedPin<'pin> {
    pub pin: Pin<'pin>,
    pub tag_list: Vec<String>,
    pub title_lowered: String,
    pub extended_lowered: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct CachedTag {
    pub tag: Tag,
    pub tag_lowered: String,
}

impl<'pin> CachedData<'pin> {
    #[allow(clippy::map_unwrap_or)]
    pub fn new<P: AsRef<Path>>(c_dir: Option<P>) -> Result<Self, Box<dyn std::error::Error>> {
        let _ = env_logger::try_init();
        debug!("new: starting");
        let cached_dir = c_dir.map(|p| p.as_ref().to_path_buf()).unwrap_or_else(|| {
            let mut dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from(""));
            dir.push(".cache");
            dir.push("rusty-pin");
            dir
        });
        debug!("  cached_dir: {:?}", cached_dir);
        let mut data = CachedData::create_cache_dir(cached_dir).map(|c_path| CachedData {
            pins: None,
            tags: None,
            tags_cache_file: c_path.join(TAGS_CACHE_FN),
            pins_cache_file: c_path.join(PINS_CACHE_FN),
            cache_dir: c_path,
            cache_files_valid: false,
        })?;

        if data.load_cache_data_from_file().is_err() {
            data.cache_files_valid = false;
        }
        Ok(data)
    }

    /// Create an instance for `CachedData` but don't load actual cached files.
    #[allow(clippy::map_unwrap_or)]
    #[allow(dead_code)]
    pub fn init<P: AsRef<Path>>(c_dir: Option<P>) -> Result<Self, Box<dyn std::error::Error>> {
        let _ = env_logger::try_init();
        debug!("init: starting");
        let cached_dir = c_dir.map(|p| p.as_ref().to_path_buf()).unwrap_or_else(|| {
            let mut dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from(""));
            dir.push(".cache");
            dir.push("rusty-pin");
            dir
        });
        debug!("  cached_dir: {:?}", cached_dir);
        let data = CachedData::create_cache_dir(cached_dir).map(|c_path| CachedData {
            pins: None,
            tags: None,
            tags_cache_file: c_path.join(TAGS_CACHE_FN),
            pins_cache_file: c_path.join(PINS_CACHE_FN),
            cache_dir: c_path,
            cache_files_valid: false,
        })?;
        Ok(data)
    }

    fn create_cache_dir<P: AsRef<Path>>(
        cache_dir: P,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let _ = env_logger::try_init();
        debug!("create_cache_dir: starting");
        fs::create_dir_all(&cache_dir)?;
        debug!(
            "  success create_cache_dir: {:?}",
            cache_dir.as_ref().to_path_buf()
        );
        Ok(cache_dir.as_ref().to_path_buf())
    }
}

impl<'pin> CachedData<'pin> {
    pub fn set_cache_dir<P: AsRef<Path>>(
        &mut self,
        p: &P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        debug!("set_cache_dir: starting");
        self.cache_dir = CachedData::create_cache_dir(p)?;
        self.tags_cache_file = self.cache_dir.join(TAGS_CACHE_FN);
        self.pins_cache_file = self.cache_dir.join(PINS_CACHE_FN);
        self.pins = None;
        self.tags = None;
        self.cache_files_valid = false;
        Ok(())
    }

    pub fn load_cache_data_from_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        debug!("load_cache_data_from_file: starting");
        match (self.tags_cache_file.exists(), self.pins_cache_file.exists()) {
            (true, true) => {
                self.read_cached_pins()?;
                self.read_cached_tags()?;
                self.cache_files_valid = true;
                Ok(())
            }
            _ => Err("Missing cache files".into()),
        }
    }

    fn read_cached_pins(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        debug!("read_cached_pins: starting");
        let fp = File::open(&self.pins_cache_file)?;
        let reader = BufReader::with_capacity(FILE_BUF_SIZE, fp);
        self.pins = rmps::from_read(reader)?;
        Ok(())
    }

    fn read_cached_tags(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        debug!("read_cached_tags: starting");
        let fp = File::open(&self.tags_cache_file)?;
        let reader = BufReader::with_capacity(FILE_BUF_SIZE, fp);
        self.tags = rmps::from_read(reader)?;
        Ok(())
    }

    pub fn cache_ok(&self) -> bool {
        debug!("cache_ok: starting");
        self.cache_files_valid
    }

    pub fn update_cache(&mut self, api: &api::Api) -> Result<(), Box<dyn std::error::Error>> {
        debug!("update_cache: starting");
        // Fetch & write all pins
        let f = File::create(&self.pins_cache_file)?;

        // Sort pins in descending creation time order
        api.all_pins()
            .map(|mut pins| {
                debug!(" sorting pins");
                pins.sort_by(|pin1, pin2| pin1.time().cmp(&pin2.time()).reverse());
                pins
            })
            .map(|pins: Vec<Pin>| {
                // Lower case all fields of each pin
                debug!(" unicode normalization and lowercasing fields");
                pins.into_iter()
                    .map(|pin| {
                        let tags_lowered = pin.tags.nfkd().collect::<String>().to_lowercase();
                        let title_lowered = pin.title.nfkd().collect::<String>().to_lowercase();
                        let extended_lowered = pin
                            .extended
                            .as_ref()
                            .map(|e| e.nfkd().collect::<String>().to_lowercase());
                        CachedPin {
                            tag_list: tags_lowered
                                .split_whitespace()
                                .map(std::string::ToString::to_string)
                                .collect(),
                            title_lowered,
                            extended_lowered,
                            pin,
                        }
                    })
                    .collect()
            })
            .and_then(|pins: Vec<CachedPin>| {
                debug!(" serializing pins");
                let mut buf: Vec<u8> = Vec::with_capacity(CACHE_BUF_SIZE);
                pins.serialize(&mut Serializer::new(&mut buf))?;
                self.pins = Some(pins);
                Ok(buf)
            })
            .and_then(|data| {
                debug!(" writing to cache");
                let mut writer = BufWriter::with_capacity(FILE_BUF_SIZE, f);
                writer.write_all(&data)?;
                Ok(())
            })?;

        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "freebsd"))]
        self.fix_cache_file_perm(&self.pins_cache_file);

        assert!(self.pins.is_some());

        // Fetch & write all tags
        //
        let f = File::create(&self.tags_cache_file)?;

        // Sort tags by frequency before writing
        api.tags_frequency()
            .map(|mut tags| {
                debug!("  sorting tags");
                tags.sort_by(|t1, t2| t1.cmp(t2).reverse());
                tags
            })
            .map(|tags| {
                debug!(" unicode normalization and lowercasing");
                tags.into_iter()
                    .map(|tag| CachedTag {
                        tag_lowered: tag.0.nfkd().collect::<String>().to_lowercase(),
                        tag,
                    })
                    .collect()
            })
            .and_then(|cached_tags: Vec<CachedTag>| {
                debug!("  serializing tags");
                let mut buf: Vec<u8> = Vec::with_capacity(CACHE_BUF_SIZE);
                cached_tags.serialize(&mut Serializer::new(&mut buf))?;
                self.tags = Some(cached_tags);
                Ok(buf)
            })
            .and_then(|data| {
                debug!("  writing to cache");
                let mut writer = BufWriter::with_capacity(FILE_BUF_SIZE, f);
                writer.write_all(&data)?;
                Ok(())
            })?;

        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "freebsd"))]
        self.fix_cache_file_perm(&self.pins_cache_file);

        assert!(self.tags.is_some());
        self.cache_files_valid = true;
        Ok(())
    }

    #[allow(clippy::unused_self)]
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "freebsd"))]
    fn fix_cache_file_perm(&self, p: &Path) {
        // TODO: don't just unwrap, return a proper error.
        use std::fs::set_permissions;
        use std::fs::Permissions;
        use std::os::unix::fs::PermissionsExt;
        debug!("fix_cache_file_perm: starting");
        let permissions = Permissions::from_mode(0o600);
        if let Err(e) = set_permissions(p, permissions) {
            error!(
                "Couldn't set proper file permission for cache files: {:?}",
                e
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rmps::Deserializer;
    use env_logger;
    use serde::Deserialize;

    #[test]
    fn unicode_normalization_test() {
        let _ = env_logger::try_init();
        debug!("serde_a_cached_pin: starting");
        let mut pin = PinBuilder::new(
            "https://danielkeep.github.io/tlborm/book/README.html",
            "The Little Book of Rust Macros آموزشی",
        )
        .tags("지구 Rust macros")
        .toread("yes")
        .shared("no")
        .description("지구")
        .into_pin();
        pin.time = Utc.ymd(2017, 5, 22).and_hms(17, 46, 54);

        let tag_list = ["지구", "Rust", "macros"]
            .iter()
            .map(|&t| t.chars().nfkd().collect::<String>().to_lowercase())
            .collect::<Vec<String>>();
        let cached_pin = CachedPin {
            pin,
            tag_list,
            title_lowered: "The Little Book of Rust Macros آموزشی"
                .chars()
                .nfkd()
                .collect::<String>()
                .to_lowercase(),
            extended_lowered: Some("지구".nfkd().collect::<String>().to_lowercase()),
        };
        // non-normalized "지구" should not be found in normalized tag list of the pin
        let r = format!(
            "\nquery: {:?}\nextended_norm: {:?}\n",
            "지구".as_bytes(),
            cached_pin.extended_lowered.as_ref().unwrap().as_bytes()
        );
        assert!(
            !cached_pin
                .extended_lowered
                .as_ref()
                .unwrap()
                .as_str()
                .contains("지구"),
            "Normalized and non-normalized tags should not be the same.{}",
            r
        );
        // normalized "지구" should be found in normalized tag list of the pin
        assert!(
            cached_pin.tag_list.contains(&"지구".into()),
            "Normalized and non-normalized tags should not be the same"
        );
        // is_nfk("آموزشی") is true, so we should be able to find it in the normalized cache.
        assert!(
            cached_pin.title_lowered.contains("آموزشی"),
            "Already normalized search queires should be found in our normalized title."
        );
        // Normalizing "آموزشی" should give us the same result as previous assert.
        let normalized_query = "آموزشی".chars().nfkd().collect::<String>();
        assert!(
            cached_pin.title_lowered.contains(&normalized_query),
            "Already normalized search queires should be found in our normalized title."
        );
    }

    #[test]
    fn serde_a_cached_pin() {
        let _ = env_logger::try_init();
        debug!("serde_a_cached_pin: starting");
        let mut pin = PinBuilder::new(
            "https://danielkeep.github.io/tlborm/book/README.html",
            "The Little Book of Rust Macros",
        )
        .tags("Rust macros")
        .toread("yes")
        .shared("no")
        .description("WoW!!!")
        .into_pin();
        pin.time = Utc.ymd(2017, 5, 22).and_hms(17, 46, 54);

        let cached_pin = CachedPin {
            pin,
            tag_list: vec!["rust".into(), "macros".into()],
            title_lowered: "The Little Book of Rust Macros".to_lowercase(),
            extended_lowered: Some("WoW!!!".to_lowercase()),
        };

        let mut buf: Vec<u8> = Vec::new();

        cached_pin
            .serialize(&mut Serializer::new(&mut buf))
            .expect("impossible");
        assert_eq!(185, buf.len());

        let mut de = Deserializer::from_read_ref(&buf);
        let new_cached: CachedPin =
            Deserialize::deserialize(&mut de).expect("Couldn't deserialize a cached pin");

        assert_eq!("The Little Book of Rust Macros", new_cached.pin.title);
        assert_eq!(
            "https://danielkeep.github.io/tlborm/book/README.html",
            new_cached.pin.url.as_ref()
        );
        assert_eq!("yes", new_cached.pin.toread);
        assert_eq!("no", new_cached.pin.shared);
        assert_eq!("WoW!!!", new_cached.pin.extended.unwrap());
        assert_eq!(
            Utc.ymd(2017, 5, 22).and_hms(17, 46, 54),
            new_cached.pin.time
        );
        assert_eq!(
            vec![String::from("rust"), String::from("macros")],
            new_cached.tag_list
        );
    }
}
