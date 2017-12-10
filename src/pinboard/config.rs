use super::*;

#[derive(Debug)]
pub struct Config {
    pub tag_only_search: bool,
    pub fuzzy_search: bool,
    pub private_new_pin: bool,
    pub toread_new_pin: bool,

    pub cache_dir: PathBuf,
    pub tags_cache_file: PathBuf,
    pub pins_cache_file: PathBuf,
}

impl Config {
    pub fn new() -> Result<Self, String> {

        fn get_app_dir() -> PathBuf {
            let mut dir = env::home_dir().unwrap_or_else(|| PathBuf::from(""));
            dir.push(".cache");
            dir.push("rusty-pin");
            dir
        }

        let cache_dir = get_app_dir();
        Config::create_cache_dir(cache_dir).and_then(|cache_dir| {
            Ok(Config {
                tag_only_search: false,
                fuzzy_search: false,
                private_new_pin: true,
                toread_new_pin: false,
                tags_cache_file: cache_dir.join("tags.cache"),
                pins_cache_file: cache_dir.join("pins.cache"),
                cache_dir,
            })
        })
    }

    pub fn set_cache_dir<P: AsRef<Path>>(&mut self, p: &P) -> Result<(), String> {
        self.cache_dir = Config::create_cache_dir(p)?;
        self.tags_cache_file = self.cache_dir.join("tags.cache");
        self.pins_cache_file = self.cache_dir.join("pins.cache");
        Ok(())
    }

//    fn enable_tag_only_search(&mut self, v: bool) {
//        self.tag_only_search = v;
//    }
//
//    fn enable_fuzzy_search(&mut self, v: bool) {
//        self.fuzzy_search = v;
//    }
//
//    fn enable_private_pin(&mut self, v: bool) {
//        self.private_new_pin = v;
//    }
//
//    fn enable_toread_pin(&mut self, v: bool) {
//        self.toread_new_pin = v;
//    }

    fn create_cache_dir<P: AsRef<Path>>(cache_dir: P) -> Result<PathBuf, String> {
        use std::fs;
        fs::create_dir_all(&cache_dir)
            .map_err(|e| e.description().to_owned())
            .and_then(|_| Ok(cache_dir.as_ref().to_path_buf()))
    }
}
