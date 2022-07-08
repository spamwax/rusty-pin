use std::borrow::Cow;

use reqwest;
use serde_json;

use chrono::prelude::*;
use url::Url;

use env_logger;

use std::collections::HashMap;
use std::io::Read;

use super::pin::Pin;
use super::tag::Tag;

use thiserror::Error;
// use serde::{Deserialize, Serialize};

#[cfg(not(test))]
const BASE_URL: &str = "https://api.pinboard.in/v1";

#[cfg(test)]
use mockito;
#[cfg(test)]
#[allow(deprecated)]
const BASE_URL: &str = mockito::SERVER_URL;

/// Struct to hold stringify results Pinboard API returns.
/// Sometimes it returns a json key of "result_code" & sometimes just "result"!!!
#[derive(Serialize, Deserialize, Debug)]
struct ApiResult {
    #[serde(default)]
    result_code: String,
    #[serde(default)]
    result: String,
}

impl ApiResult {
    fn ok(self) -> Result<(), Box<dyn std::error::Error>> {
        if self.result_code == "done" || self.result == "done" {
            Ok(())
        } else if !self.result_code.is_empty() {
            Err(self.result_code.into())
        } else {
            Err(self.result.into())
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateTime {
    #[serde(rename = "update_time")]
    datetime: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Api<'api> {
    auth_token: Cow<'api, str>,
}

#[derive(Debug, Error)]
pub enum ApiError {
    // #[fail(display = "invalid url: {}", _0)]
    #[error("invalid url: {0}")]
    UrlError(String),
    // #[fail(display = "invalid server response: {}", _0)]
    #[error("invalid server response: {0}")]
    UnrecognizedResponse(String),
    // #[fail(display = "Server couldn't fulfill request: {}", _0)]
    #[error("server couldn't fulfill request: {0}")]
    ServerError(String),
    // #[fail(display = "network error: {}", _0)]
    #[error("network error: {0}")]
    Network(String),
    // Network(#[from] std::io::Error),
    // #[fail(display = "serde error: {}", _0)]
    #[error("serde error: {0}")]
    SerdeError(String),
}

impl<'api, 'pin> Api<'api> {
    pub fn new<S>(auth_token: S) -> Self
    where
        S: Into<Cow<'api, str>>,
    {
        let _ = env_logger::try_init();
        Api {
            auth_token: auth_token.into(),
        }
    }

    pub fn all_pins(&self) -> Result<Vec<Pin<'pin>>, Box<dyn std::error::Error>> {
        debug!("all_pins: starting.");
        let res =
            self.get_api_response([BASE_URL, "/posts/all"].concat().as_str(), HashMap::new())?;
        debug!("  received all bookmarks");

        let mut v: serde_json::Value =
            serde_json::from_str(res.as_str()).map_err(|e| ApiError::SerdeError(e.to_string()))?;
        let v = v.as_array_mut().ok_or_else(|| {
            ApiError::UnrecognizedResponse("array of bookmarks expected from server".to_string())
        })?;

        let v_len = v.len();

        let pins: Vec<Pin> = v
            .drain(..)
            .filter_map(|line| serde_json::from_value(line).ok())
            .filter(|p: &Pin| Url::parse(&p.url).is_ok())
            .collect();
        if pins.len() != v_len {
            info!(
                "couldn't parse {} bookmarks (out of {})",
                v_len - pins.len(),
                v_len
            );
        } else {
            info!("parsed all bookmarks. total: {}", pins.len());
        }

        Ok(pins)
    }

    pub fn suggest_tags<T: AsRef<str>>(
        &self,
        url: T,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        debug!("suggest_tags: starting.");
        let mut query = HashMap::new();
        query.insert("url", url.as_ref());

        Ok(self
            .get_api_response([BASE_URL, "/posts/suggest"].concat().as_str(), query)
            .and_then(|res| {
                serde_json::from_str::<Vec<serde_json::Value>>(&res)
                    .map_err(|e| ApiError::SerdeError(e.to_string()).into())
            })?
            .into_iter()
            .find(|item| !item["popular"].is_null())
            .map(|item| {
                item["popular"]
                    .as_array()
                    .unwrap_or(&vec![json!([])])
                    .iter()
                    .map(|v| v.as_str().unwrap_or("").to_string())
                    .collect::<Vec<String>>()
            })
            .ok_or_else(|| {
                ApiError::UnrecognizedResponse(
                    "Unrecognized response from API: posts/suggest".to_string(),
                )
            })?)
    }

    pub fn add_url(&self, p: Pin) -> Result<(), Box<dyn std::error::Error>> {
        debug!("add_url: starting.");
        let url: &str = &p.url;
        let extended = &p.extended.unwrap_or_default();
        let mut map = HashMap::new();
        debug!(" url: {}", url);

        map.insert("url", url);
        map.insert("description", &p.title);
        map.insert("tags", &p.tags);
        map.insert("toread", &p.toread);
        map.insert("extended", extended);
        map.insert("shared", &p.shared);
        map.insert("replace", "yes");

        debug!("Sending payload to: {}/posts/add\n\t{:?}", BASE_URL, map);
        self.get_api_response([BASE_URL, "/posts/add"].concat().as_str(), map)
            .and_then(|res| {
                serde_json::from_str::<ApiResult>(&res)
                    .map_err(|e| From::from(ApiError::UnrecognizedResponse(e.to_string())))
            })
            .and_then(self::ApiResult::ok)
    }

    pub fn tag_rename<T: AsRef<str>>(
        &self,
        old: T,
        new: T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        debug!("tag_rename: starting.");
        let mut map = HashMap::new();
        map.insert("old", old.as_ref());
        map.insert("new", new.as_ref());
        self.get_api_response([BASE_URL, "/tags/rename"].concat(), map)
            .and_then(|res| {
                serde_json::from_str::<ApiResult>(&res)
                    .map_err(|e| From::from(ApiError::UnrecognizedResponse(e.to_string())))
            })
            .and_then(self::ApiResult::ok)
    }

    pub fn tag_delete<T: AsRef<str>>(&self, tag: T) -> Result<(), Box<dyn std::error::Error>> {
        debug!("tag_rename: starting.");
        let mut map = HashMap::new();
        map.insert("tag", tag.as_ref());
        self.get_api_response([BASE_URL, "/tags/delete"].concat(), map)
            .and_then(|res| {
                serde_json::from_str::<ApiResult>(&res)
                    .map_err(|e| From::from(ApiError::UnrecognizedResponse(e.to_string())))
            })
            .and_then(self::ApiResult::ok)
    }

    /// Gets all tags with their usage frequency.
    pub fn tags_frequency(&self) -> Result<Vec<Tag>, Box<dyn std::error::Error>> {
        // Pinboard API returns json narray when user has no tags, otherwise it returns an
        // object/map of tag:frequency!
        debug!("tags_frequency: starting.");
        let res =
            self.get_api_response([BASE_URL, "/tags/get"].concat().as_str(), HashMap::new())?;
        // Assuming pinboard is returing String:number style for tag frequency
        debug!("  trying string:usize map");
        let tag_freq = serde_json::from_str::<HashMap<String, usize>>(&res)
            .map(|tagmap| {
                tagmap
                    .into_iter()
                    .map(|(tag, freq)| Tag::new(tag, freq))
                    .collect()
            })
            .map_err(|e| e.into());
        if tag_freq.is_ok() {
            return tag_freq;
        }
        // Assuming pinboard has returned String:String style for tag frequency since last try didn't work
        debug!("  trying string:string map");
        let tag_freq = serde_json::from_str::<HashMap<String, String>>(&res)
            .map(|tagmap| {
                tagmap
                    .into_iter()
                    .map(|(k, v)| {
                        let freq = v.parse::<usize>().unwrap_or_default();
                        Tag::new(k, freq)
                    })
                    .collect()
            })
            .map_err(|e| e.into());
        if tag_freq.is_ok() {
            return tag_freq;
        }
        // If we are here, it most likely means that user's tag list is empty and pinboard is returning
        // an empty vector instead of an object
        debug!("   couldn't get a tag2freq map");
        debug!("   {:?}", tag_freq);
        debug!("  trying to decode non-object empty tag list");
        let raw_tags = serde_json::from_str::<Vec<HashMap<String, String>>>(&res)?;
        assert!(raw_tags.is_empty());
        Ok(vec![])
    }

    pub fn delete<T: AsRef<str>>(&self, url: T) -> Result<(), Box<dyn std::error::Error>> {
        debug!("delete: starting.");
        let mut map = HashMap::new();
        debug!(" url: {}", url.as_ref());
        map.insert("url", url.as_ref());

        self.get_api_response([BASE_URL, "/posts/delete"].concat().as_str(), map)
            .and_then(|res| {
                serde_json::from_str(&res)
                    .map_err(|e| From::from(ApiError::UnrecognizedResponse(e.to_string())))
            })
            .and_then(self::ApiResult::ok)
    }

    pub fn recent_update(&self) -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
        debug!("recent_update: starting.");
        self.get_api_response(
            [BASE_URL, "/posts/update"].concat().as_str(),
            HashMap::new(),
        )
        .and_then(|res| {
            serde_json::from_str(&res).map_err(|e| From::from(ApiError::SerdeError(e.to_string())))
        })
        .map(|date: UpdateTime| date.datetime)
    }

    fn add_auth_token<T: AsRef<str>>(&self, url: T) -> Url {
        debug!("add_auth_token: starting.");
        // debug!("  token: `{}`", &self.auth_token);
        Url::parse_with_params(
            url.as_ref(),
            &[("format", "json"), ("auth_token", &self.auth_token)],
        )
        .expect("invalid parameters")
    }

    fn get_api_response<T: AsRef<str>>(
        &self,
        endpoint: T,
        params: HashMap<&str, &str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        debug!("get_api_response: starting.");

        let endpoint_string = endpoint.as_ref().to_string();
        let mut base_url =
            Url::parse(endpoint.as_ref()).map_err(|_| ApiError::UrlError(endpoint_string))?;
        debug!("  url: {:?}", base_url);

        for (k, v) in params {
            base_url.query_pairs_mut().append_pair(k, v);
        }
        let api_url = self.add_auth_token(base_url);

        let client = reqwest::blocking::Client::new();
        let r = client.get(api_url).send();

        match r {
            Err(e) => {
                if e.is_connect() {
                    return Err(Box::new(ApiError::Network(e.to_string())));
                } else {
                    return Err(Box::new(ApiError::UnrecognizedResponse(e.to_string())));
                }
            }
            Ok(_) => {
                debug!("  server resp is ok (no error)");
            }
        }
        let mut resp = r.unwrap();

        if resp.status().is_success() {
            let mut content = String::with_capacity(2 * 1024);
            let _bytes_read = resp.read_to_string(&mut content)?;
            debug!(" string from resp ok");
            debug!("   {:?}", content.chars().take(15).collect::<Vec<char>>());
            debug!(" returning from get_api_response");
            Ok(content)
        } else {
            debug!("  response status indicates error");
            debug!("    {:?}", resp.status().as_str());
            debug!("    {:?}", resp.status().canonical_reason(),);
            let e = ApiError::ServerError(
                resp.status()
                    .canonical_reason()
                    .expect("UNKNOWN RESPONSE")
                    .to_string(),
            )
            .into();
            debug!("    ERR: {:?}", e);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::pinboard::mockito_helper::start_mockito_server;
    use crate::pinboard::mockito_helper::MockBodyGenerate;
    use crate::pinboard::pin::PinBuilder;
    use crate::pinboard::tag;

    const TEST_URL: &str = "https://githuуй.com/Здравствуйт?q=13#fragment";
    #[test]
    fn get_latest_update_time() {
        let _ = env_logger::try_init();
        debug!("get_latest_update_time: starting.");
        let _m = start_mockito_server(
            r"^/posts/update.*$",
            200,
            r#"{"update_time":"2018-02-07T01:54:09Z"}"#,
        );
        let api = Api::new(include_str!("api_token.txt"));
        let r = api.recent_update();
        assert!(r.is_ok());
    }

    // This test will always panic if the BASE_URL is not set to some unreachable address causes
    // a network error.
    // To actually test network errors, change the BASE_URL to something like
    // "http://oaeisn13k.com" and remove the should_panic attribute. In that case the test should
    // pass sicne we will encounter a network error.
    #[test]
    #[should_panic(expected = "Expected ApiError::Network")]
    fn network_io_error_test() {
        let _m1 = start_mockito_server(r"^/posts/delete.*$", 429, r#"io error"#);
        let api = Api::new(include_str!("api_token.txt"));
        let r = api.delete("http://google.com/public");
        assert!(r.is_err());
        let err = r.unwrap_err();
        let err = err.downcast_ref::<ApiError>();
        match err.unwrap() {
            ApiError::Network(_) => println!("GOT Network"),
            _ => panic!("Expected ApiError::Network"),
        }
    }

    #[test]
    fn too_many_requests() {
        let _m1 = start_mockito_server(r"^/posts/delete.*$", 429, r#"Back off"#);
        let api = Api::new(include_str!("api_token.txt"));
        let r = api.delete(TEST_URL);
        assert_eq!(
            "server couldn't fulfill request: Too Many Requests",
            r.expect_err("Expected Not Found").to_string()
        );
    }

    #[test]
    fn delete_tag_test() {
        let _ = env_logger::try_init();
        debug!("delete_tag_test: starting.");
        let _m1 = start_mockito_server(r#"^/tags/delete.*$"#, 200, r#"{"result":"done"}"#);
        let api = Api::new(include_str!("api_token.txt"));
        let r = api.tag_delete("DUMMY");
        r.expect("Error in deleting a tag.");

        {
            // Deleting non-existing tag
            // Pinboard returns OK on this operation!!!
            let _m2 = start_mockito_server(
                r"^/tags/delete.+fucking\.way.*$",
                200,
                r#"{"result":"done"}"#,
            );
            let _ = api
                .tag_delete("http://no.fucking.way")
                .expect("pinboard OKs deleting a non-existing tag.");
        }

        {
            // Deleting empty string
            // Pinboard returns OK on this operation!!!
            let _m2 = start_mockito_server(r"^/tags/delete.*$", 200, r#"{"result":"done"}"#);
            let _ = api
                .tag_delete("")
                .expect("pinboard OKs deleting a non-existing tag.");
        }
    }

    #[test]
    fn rename_tag_test() {
        let _ = env_logger::try_init();
        debug!("rename_tag_test: starting");
        let _m1 = start_mockito_server(r#"^/tags/rename.*$"#, 200, r#"{"result":"done"}"#);
        let api = Api::new(include_str!("api_token.txt"));
        let r = api.tag_rename("old_tag", "new_tag");
        r.expect("Error in renaming a tag.");

        // Pinboard apparently can rename null to a new tag!!!
        let _ = api
            .tag_rename("", "iamjesus")
            .expect("Should be able to breath life into abyss");

        {
            // renaming to an empty tag
            let _m2 =
                start_mockito_server(r#"^/tags/rename.*$"#, 200, r#"{"result":"rename to null"}"#);
            let r = api
                .tag_rename("old_tag", "")
                .expect_err("renaming to empty tag should return error");
            assert_eq!("rename to null".to_string(), r.to_string());
        }
    }

    #[test]
    fn delete_api_test() {
        let _ = env_logger::try_init();
        debug!("delete_a_pin: starting.");
        add_a_url();
        let _m1 = start_mockito_server(r#"^/posts/delete.*$"#, 200, r#"{"result_code":"done"}"#);
        let api = Api::new(include_str!("api_token.txt"));
        let r = api.delete(TEST_URL);
        r.expect("Error in deleting a pin.");

        {
            // Deleting non-existing bookmark
            let _m2 = start_mockito_server(
                r"^/posts/delete.+fucking\.way.*$",
                200,
                r#"{"result_code":"item not found"}"#,
            );
            let r = api
                .delete("http://no.fucking.way")
                .expect_err("Deleted non-existing pin");
            assert_eq!("item not found".to_string(), r.to_string());
        }

        {
            // Deleting malformed url
            let _m2 = start_mockito_server(
                r"^/posts/delete.*$",
                200,
                r#"{"result_code":"item not found"}"#,
            );
            let r = api
                .delete(":// bad url/#")
                .expect_err("should not find a malformed url to delete");
            assert_eq!("item not found".to_string(), r.to_string());
        }
    }

    #[test]
    fn add_a_url() {
        let _ = env_logger::try_init();
        debug!("add_a_url: starting.");
        let _m1 = start_mockito_server(r"^/posts/add.*$", 200, r#"{"result_code":"done"}"#);
        let api = Api::new(include_str!("api_token.txt"));
        let p = PinBuilder::new(TEST_URL, "test bookmark/pin")
            .tags("tagestan what")
            .description("russian website!")
            .shared("yes")
            .into_pin();
        let res = api.add_url(p);
        res.expect("Error in adding a pin.");

        {
            // Adding a malformed url
            let _m1 = start_mockito_server(
                r"^/posts/add.+bad_url.*$",
                200,
                r#"{"result_code":"missing url"}"#,
            );
            let p = PinBuilder::new(":// bad_url/#", "test bookmark/pin")
                .tags("tagestan what")
                .description("russian website!")
                .shared("yes")
                .into_pin();
            let r = api
                .add_url(p)
                .expect_err("server should not have accepted malformed url");
            assert_eq!("missing url", r.to_string());
        }
    }

    #[test]
    fn suggest_tags() {
        let _ = env_logger::try_init();
        debug!("suggest_tags: starting.");
        let _m1 = start_mockito_server(
            r"^/posts/suggest.*$",
            200,
            PathBuf::from("tests/suggested_tags_mockito.json"),
        );
        let api = Api::new(include_str!("api_token.txt"));
        let url = "http://blog.com/";
        let res = api.suggest_tags(url);
        assert_eq!(
            vec!["datetime", "library", "rust"],
            res.expect("impossible")
        );
    }

    #[test]
    fn test_tag_freq_str2str() {
        let _ = env_logger::try_init();
        debug!("test_tag_freq: starting.");
        let _m1 = PathBuf::from("tests/all_tags_mockito.json")
            .create_mockito_server(r"^/tags/get.*$", 200);
        let api = Api::new(include_str!("api_token.txt"));
        let res = api.tags_frequency();
        let r = res.unwrap_or_else(|e| panic!("{:?}", e));
        assert_eq!(94, r.len());
    }

    #[test]
    fn test_tag_freq_str2int() {
        let _ = env_logger::try_init();
        debug!("test_tag_freq: starting.");
        let _m1 = PathBuf::from("tests/all_tags_mockito2.json")
            .create_mockito_server(r"^/tags/get.*$", 200);
        let api = Api::new(include_str!("api_token.txt"));
        let res = api.tags_frequency();
        let r = res.unwrap_or_else(|e| panic!("{:?}", e));
        if let Some(tag) = r.iter().find(|&t| t.0 == "آموزشی") {
            match tag.1 {
                tag::TagFreq::Used(5) => {}
                _ => panic!("Expecetd tag freuqency of 5, got {:?}", tag.1),
            }
        } else {
            panic!("Can't find a specific tag in mock list of tag frequencies");
        }
        assert_eq!(94, r.len());
    }

    #[test]
    fn test_tag_freq_empty() {
        let _ = env_logger::try_init();
        debug!("test_tag_freq_empty: starting.");
        {
            let _m1 = "[]".create_mockito_server(r"^/tags/get.*$", 201);
            let api = Api::new(include_str!("api_token.txt"));
            let res = api.tags_frequency();
            let r = res.unwrap_or_else(|e| panic!("{:?}", e));
            assert!(r.is_empty());
        }
        {
            let _m1 = "{}".create_mockito_server(r"^/tags/get.*$", 201);
            let api = Api::new(include_str!("api_token.txt"));
            let res = api.tags_frequency();
            let r = res.unwrap_or_else(|e| panic!("{:?}", e));
            assert!(r.is_empty());
        }
    }

    #[test]
    fn test_all_pins() {
        let _ = env_logger::try_init();
        debug!("test_all_pins: starting.");
        let _m1 = start_mockito_server(
            r"^/posts/all.*$",
            200,
            PathBuf::from("tests/all_pins_mockito.json"),
        );
        let api = Api::new(include_str!("api_token.txt"));
        let res = api.all_pins();

        assert_eq!(58, res.unwrap_or_else(|e| panic!("{:?}", e)).len());
    }

    #[test]
    fn test_all_pins_empty() {
        let _ = env_logger::try_init();
        debug!("test_all_pins: starting.");
        {
            let _m1 = "[]".create_mockito_server(r"^/posts/all.*$", 200);
            let api = Api::new(include_str!("api_token.txt"));
            let res = api.all_pins();

            assert_eq!(0, res.unwrap_or_else(|e| panic!("{:?}", e)).len());
        }
    }
}
