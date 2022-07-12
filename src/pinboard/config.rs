#[allow(clippy::struct_excessive_bools)]
#[derive(Debug)]
#[non_exhaustive]
pub struct Config {
    pub tag_only_search: bool,
    pub fuzzy_search: bool,
    pub private_new_pin: bool,
    pub toread_new_pin: bool,
    // _private: (), // Force instantiation through Config::new()
}

impl Config {
    pub fn new() -> Self {
        Config {
            tag_only_search: false,
            fuzzy_search: false,
            private_new_pin: true,
            toread_new_pin: false,
            // _private: (),
        }
    }
}
