use serde_json;
use url_serde;
use reqwest;
use reqwest::IntoUrl;

use chrono::prelude::*;
use url::Url;

use Pin;
mod api;

#[derive(Debug)]
pub struct Pinboard {
    auth_token: String,
}

#[derive(Debug)]
struct PinBuilder {
    pin: Pin,
}

impl PinBuilder {
    fn new<T: IntoUrl>(url: T, title: String) -> Self {
        PinBuilder { pin: Pin::new(url.into_url().unwrap(), title, vec![], true, false, None) }
    }
}

impl PinBuilder {
    fn tags(t: String) -> Self {
        unimplemented!();
    }
}
impl Pinboard {
    pub fn new(auth_token: String) -> Self {
        Pinboard { auth_token }
    }
}
