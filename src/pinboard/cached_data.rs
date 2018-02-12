use super::*;
use std::io::Write;
use std::io::{BufReader, BufWriter};
use env_logger;

use serde::Serialize;

use failure::Error;

use super::pin::{Pin, Tag};

const TAGS_CACHE_FN: &str = "tags.cache";
const PINS_CACHE_FN: &str = "pins.cache";

const FILE_BUF_SIZE: usize = 4 * 1024 * 1024;
const CACHE_BUF_SIZE: usize = 1024;

#[derive(Debug)]
pub struct CachedData {
    pub pins: Option<Vec<CachedPin>>,
    pub tags: Option<Vec<Tag>>,
    pub cache_dir: PathBuf,
    pub tags_cache_file: PathBuf,
    pub pins_cache_file: PathBuf,
    cache_files_valid: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CachedPin {
    pub pin: Pin,
    pub tag_list: Vec<String>,
}

impl CachedData {
    pub fn new<P: AsRef<Path>>(c_dir: Option<P>) -> Result<Self, Error> {
        let _ = env_logger::try_init();
        info!("new: starting");
        let cached_dir = c_dir.map(|p| p.as_ref().to_path_buf()).unwrap_or_else(|| {
            let mut dir = env::home_dir().unwrap_or_else(|| PathBuf::from(""));
            dir.push(".cache");
            dir.push("rusty-pin");
            dir
        });
        let mut data = CachedData::create_cache_dir(cached_dir).and_then(|c_path| {
            Ok(CachedData {
                pins: None,
                tags: None,
                tags_cache_file: c_path.join(TAGS_CACHE_FN),
                pins_cache_file: c_path.join(PINS_CACHE_FN),
                cache_dir: c_path,
                cache_files_valid: false,
            })
        })?;

        if data.load_cache_data_from_file().is_err() {
            data.cache_files_valid = false;
        }
        Ok(data)
    }

    /// Create an instance for CachedData but don't load actual cached files.
    pub fn init<P: AsRef<Path>>(c_dir: Option<P>) -> Result<Self, Error> {
        let _ = env_logger::try_init();
        info!("init: starting");
        let cached_dir = c_dir.map(|p| p.as_ref().to_path_buf()).unwrap_or_else(|| {
            let mut dir = env::home_dir().unwrap_or_else(|| PathBuf::from(""));
            dir.push(".cache");
            dir.push("rusty-pin");
            dir
        });
        let data = CachedData::create_cache_dir(cached_dir).and_then(|c_path| {
            Ok(CachedData {
                pins: None,
                tags: None,
                tags_cache_file: c_path.join(TAGS_CACHE_FN),
                pins_cache_file: c_path.join(PINS_CACHE_FN),
                cache_dir: c_path,
                cache_files_valid: false,
            })
        })?;
        Ok(data)
    }

    fn create_cache_dir<P: AsRef<Path>>(cache_dir: P) -> Result<PathBuf, Error> {
        let _ = env_logger::try_init();
        info!("create_cache_dir: starting");
        use std::fs;
        fs::create_dir_all(&cache_dir)?;
        Ok(cache_dir.as_ref().to_path_buf())
    }
}

impl CachedData {
    pub fn set_cache_dir<P: AsRef<Path>>(&mut self, p: &P) -> Result<(), Error> {
        info!("set_cache_dir: starting");
        self.cache_dir = CachedData::create_cache_dir(p)?;
        self.tags_cache_file = self.cache_dir.join(TAGS_CACHE_FN);
        self.pins_cache_file = self.cache_dir.join(PINS_CACHE_FN);
        self.pins = None;
        self.tags = None;
        self.cache_files_valid = false;
        Ok(())
    }

    pub fn load_cache_data_from_file(&mut self) -> Result<(), Error> {
        info!("load_cache_data_from_file: starting");
        match (self.tags_cache_file.exists(), self.pins_cache_file.exists()) {
            (true, true) => {
                self.read_cached_pins()?;
                self.read_cached_tags()?;
                self.cache_files_valid = true;
                Ok(())
            }
            _ => bail!("Missing cache files"),
        }
    }

    fn read_cached_pins(&mut self) -> Result<(), Error> {
        info!("read_cached_pins: starting");
        let fp = File::open(&self.pins_cache_file)?;
        let reader = BufReader::with_capacity(FILE_BUF_SIZE, fp);
        let mut de = Deserializer::from_read(reader);
        self.pins = Deserialize::deserialize(&mut de)?;
        Ok(())
    }

    fn read_cached_tags(&mut self) -> Result<(), Error> {
        info!("read_cached_tags: starting");
        let fp = File::open(&self.tags_cache_file)?;
        let reader = BufReader::with_capacity(FILE_BUF_SIZE, fp);
        let mut de = Deserializer::from_read(reader);
        self.tags = Deserialize::deserialize(&mut de)?;
        Ok(())
    }

    pub fn cache_ok(&self) -> bool {
        info!("cache_ok: starting");
        self.cache_files_valid
    }

    pub fn update_cache(&mut self, api: &api::Api) -> Result<(), Error> {
        info!("update_cache: starting");
        // Fetch & write all pins
        let f = File::create(&self.pins_cache_file)?;

        // Sort pins in descending creation time order
        api.all_pins()
            .and_then(|mut pins| {
                pins.sort_by(|pin1, pin2| pin1.time().cmp(&pin2.time()).reverse());
                Ok(pins)
            })
            .and_then(|pins: Vec<Pin>| {
                // Lower case all fields of each pin
                Ok(pins.into_iter()
                    .map(|pin| {
                        let url_lowered = Url::parse(pin.url.as_str())
                            .expect("Invalid url stored in a pin, impossible?");
                        let mut pb = PinBuilder::new(url_lowered, pin.title.to_lowercase())
                            .tags(pin.tags.to_lowercase())
                            .shared(&pin.shared)
                            .toread(&pin.toread);
                        if pin.extended.is_some() {
                            pb = pb.description(pin.extended.map(|s| s.to_lowercase()).unwrap());
                        }
                        let mut newpin = pb.into_pin();
                        newpin.time = pin.time;
                        CachedPin {
                            pin: newpin,
                            tag_list: pin.tags.split_whitespace().map(|s| s.to_string()).collect(),
                        }
                    })
                    .collect())
            })
            .and_then(|pins: Vec<CachedPin>| {
                let mut buf: Vec<u8> = Vec::with_capacity(CACHE_BUF_SIZE);
                pins.serialize(&mut Serializer::new(&mut buf))?;
                self.pins = Some(pins);
                Ok(buf)
            })
            .and_then(|data| {
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
            .and_then(|mut tags| {
                tags.sort_by(|t1, t2| {
                    if t1.1 != t2.1 {
                        t1.1.cmp(&t2.1).reverse()
                    } else {
                        t1.0.cmp(&t2.0)
                    }
                });
                Ok(tags)
            })
            .and_then(|tags_tuple| {
                let mut buf: Vec<u8> = Vec::with_capacity(CACHE_BUF_SIZE);
                tags_tuple.serialize(&mut Serializer::new(&mut buf))?;
                self.tags = Some(tags_tuple);
                Ok(buf)
            })
            .and_then(|data| {
                let mut writer = BufWriter::with_capacity(FILE_BUF_SIZE, f);
                writer.write_all(&data)?; // write_all resturn Result<()>
                Ok(())
            })?;

        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "freebsd"))]
        self.fix_cache_file_perm(&self.pins_cache_file);

        assert!(self.tags.is_some());
        self.cache_files_valid = true;
        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "freebsd"))]
    fn fix_cache_file_perm(&self, p: &PathBuf) {
        info!("fix_cache_file_perm: starting");
        // TODO: don't just unwrap, return a proper error.
        use std::fs::Permissions;
        use std::os::unix::fs::PermissionsExt;
        use std::fs::set_permissions;
        let permissions = Permissions::from_mode(0o600);
        set_permissions(p, permissions)
            .map_err(|e| e.to_string())
            .expect("Couludn't set file permissiion");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use env_logger;

    #[test]
    fn serde_a_cached_pin() {
        let _ = env_logger::try_init();
        info!("serde_a_cached_pin: starting");
        let mut pin = PinBuilder::new(
            "https://danielkeep.github.io/tlborm/book/README.html",
            "The Little Book of Rust Macros".to_string(),
        ).tags("Rust macros".to_string())
            .toread("yes")
            .shared("no")
            .description("WoW!!!".to_string())
            .into_pin();
        pin.time = Utc.ymd(2017, 5, 22).and_hms(17, 46, 54);

        let cached_pin = CachedPin {
            pin: pin,
            tag_list: vec!["Rust".into(), "macros".into()],
        };

        let mut buf: Vec<u8> = Vec::new();

        cached_pin
            .serialize(&mut Serializer::new(&mut buf))
            .unwrap();
        assert_eq!(147, buf.len());

        let mut de = Deserializer::from_slice(&buf);
        let new_cached: CachedPin = Deserialize::deserialize(&mut de).unwrap();

        assert_eq!(
            "The Little Book of Rust Macros".to_string(),
            new_cached.pin.title
        );
        assert_eq!(
            "https://danielkeep.github.io/tlborm/book/README.html",
            new_cached.pin.url.as_ref()
        );
        assert_eq!("yes".to_string(), new_cached.pin.toread);
        assert_eq!("no".to_string(), new_cached.pin.shared);
        assert_eq!("WoW!!!".to_string(), new_cached.pin.extended.unwrap());
        assert_eq!(
            Utc.ymd(2017, 5, 22).and_hms(17, 46, 54),
            new_cached.pin.time
        );
        assert_eq!(
            vec!["Rust".to_string(), "macros".to_string()],
            new_cached.tag_list
        );
    }

}
