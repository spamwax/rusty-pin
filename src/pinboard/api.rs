use std::borrow::Cow;

use reqwest;
use reqwest::IntoUrl;
use serde_json;

use chrono::prelude::*;
use url::Url;

use env_logger;

use std::collections::HashMap;
use std::io::Read;

use failure::{err_msg, Error};

use super::pin::{Pin, Tag};

#[cfg(not(test))]
const BASE_URL: &str = "https://api.pinboard.in/v1";

#[cfg(test)]
use mockito;
#[cfg(test)]
const BASE_URL: &str = mockito::SERVER_URL;

#[derive(Serialize, Deserialize, Debug)]
struct ApiResult {
    result_code: String,
}

impl ApiResult {
    fn ok(self) -> Result<(), Error> {
        if self.result_code == "done" {
            Ok(())
        } else {
            bail!(self.result_code)
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

#[derive(Debug, Fail)]
pub enum ApiError {
    #[fail(display = "invalid url: {}", _0)]
    UrlError(String),
    #[fail(display = "invalid server response: {}", _0)]
    UnrecognizedResponse(String),
    #[fail(display = "Server couldn't fulfill request: {}", _0)]
    ServerError(String),
    #[fail(display = "network error: {}", _0)]
    Network(String),
    #[fail(display = "serde error: {}", _0)]
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

    pub fn all_pins(&self) -> Result<Vec<Pin<'pin>>, Error> {
        debug!("all_pins: starting.");
        let res = self.get_api_response([BASE_URL, "/posts/all"].concat().as_str(), HashMap::new())
            .unwrap();
        debug!("  received all bookmarks");
        let pins =
            serde_json::from_str(&res).map_err(|e| From::from(ApiError::SerdeError(e.to_string())));
        if pins.is_err() {
            debug!("  couldn't deserialize bookmarks.");
        } else {
            debug!("  deserialized received bookmarks");
        }
        pins
    }

    pub fn suggest_tags<T: IntoUrl>(&self, url: T) -> Result<Vec<String>, Error> {
        debug!("suggest_tags: starting.");
        let u: &str = &url.into_url()?.to_string();
        let mut query = HashMap::new();
        query.insert("url", u);

        self.get_api_response([BASE_URL, "/posts/suggest"].concat().as_str(), query)
            .and_then(|res| {
                serde_json::from_str::<Vec<serde_json::Value>>(&res)
                    .map_err(|e| From::from(ApiError::SerdeError(e.to_string())))
            })?
            .into_iter()
            .find(|item| !item["popular"].is_null())
            .map(|item| {
                item["popular"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect::<Vec<String>>()
            })
            .ok_or_else(|| {
                From::from(ApiError::UnrecognizedResponse(
                    "Unrecognized response from API: posts/suggest".to_string(),
                ))
            })
    }

    pub fn add_url(&self, p: Pin) -> Result<(), Error> {
        debug!("add_url: starting.");
        let url: &str = &p.url.into_string();
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
            .and_then(|r| r.ok())
    }

    pub fn tags_frequency(&self) -> Result<Vec<Tag>, Error> {
        debug!("tags_frequency: starting.");
        self.get_api_response([BASE_URL, "/tags/get"].concat().as_str(), HashMap::new())
            .and_then(|res| {
                serde_json::from_str(&res)
                    .map_err(|e| From::from(ApiError::SerdeError(e.to_string())))
            })
            .and_then(|res: HashMap<String, String>| {
                Ok(res.into_iter()
                    .map(|(k, v)| {
                        let freq = v.parse::<usize>().unwrap_or_default();
                        Tag(k, freq)
                    })
                    .collect())
            })
    }

    pub fn delete<T: IntoUrl>(&self, url: T) -> Result<(), Error> {
        debug!("delete: starting.");
        let url: &str = &url.into_url()?.to_string();
        let mut map = HashMap::new();
        debug!(" url: {}", url);
        map.insert("url", url);

        self.get_api_response([BASE_URL, "/posts/delete"].concat().as_str(), map)
            .and_then(|res| {
                serde_json::from_str(&res)
                    .map_err(|e| From::from(ApiError::UnrecognizedResponse(e.to_string())))
            })
            .and_then(|r: ApiResult| r.ok())
    }

    pub fn recent_update(&self) -> Result<DateTime<Utc>, Error> {
        debug!("recent_update: starting.");
        self.get_api_response(
            [BASE_URL, "/posts/update"].concat().as_str(),
            HashMap::new(),
        ).and_then(|res| {
                serde_json::from_str(&res)
                    .map_err(|e| From::from(ApiError::SerdeError(e.to_string())))
            })
            .and_then(|date: UpdateTime| Ok(date.datetime))
    }

    fn add_auth_token<T: IntoUrl>(&self, url: T) -> Url {
        debug!("add_auth_token: starting.");
        debug!("  token: `{}`", &self.auth_token);
        let u = Url::parse_with_params(
            url.into_url().expect("invalid url").as_ref(),
            &[("format", "json"), ("auth_token", &self.auth_token)],
        ).expect("invalid parameters");
        debug!("  url: {:?}", u);
        u
    }

    fn get_api_response<T: IntoUrl + AsRef<str>>(
        &self,
        endpoint: T,
        params: HashMap<&str, &str>,
    ) -> Result<String, Error> {
        debug!("get_api_response: starting.");

        let endpoint_string = endpoint.as_ref().to_string();
        let mut base_url = endpoint.into_url().map_err(|_| {
            let api_err: Error = From::from(ApiError::UrlError(endpoint_string));
            api_err
        })?;

        for (k, v) in params {
            base_url.query_pairs_mut().append_pair(k, v);
        }
        debug!("  no-auth url: {:?}", base_url);
        let api_url = self.add_auth_token(base_url);

        let client = reqwest::Client::new();
        let r = client.get(api_url).send();

        let mut resp = r.map_err(|e| {
            use std::io;
            if e.get_ref()
                .and_then(|k| k.downcast_ref::<io::Error>())
                .is_some()
            {
                err_msg("Network IO error")
            } else {
                use std::error::Error as StdError;
                let api_err: Error = From::from(ApiError::Network(format!(
                    "Network request error: {:?}",
                    e.description()
                )));
                api_err
            }
        })?;
        debug!(" resp is ok (no error)");

        if resp.status().is_success() {
            let mut content = String::with_capacity(2 * 1024);
            let _bytes_read = resp.read_to_string(&mut content)?;
            debug!(" string from resp ok");
            Ok(content)
        } else {
            debug!(" response status indicates error");
            Err(From::from(ApiError::ServerError(
                resp.status()
                    .canonical_reason()
                    .expect("UNKNOWN RESPONSE")
                    .to_string(),
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use url::ParseError;

    use pinboard::mockito_helper::start_mockito_server;
    use pinboard::pin::PinBuilder;

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

    #[test]
    fn too_many_requests() {
        let _m1 = start_mockito_server(r"^/posts/delete.*$", 429, r#"Back off"#);
        let api = Api::new(include_str!("api_token.txt"));
        let r = api.delete(TEST_URL);
        assert_eq!(
            "Server couldn't fulfill request: Too Many Requests",
            r.expect_err("Expected Not Found").root_cause().to_string()
        );
    }

    #[test]
    fn delete_a_pin() {
        let _ = env_logger::try_init();
        debug!("delete_a_pin: starting.");
        add_a_url();
        let _m1 = start_mockito_server(r#"^/posts/delete.*$"#, 200, r#"{"result_code":"done"}"#);
        let api = Api::new(include_str!("api_token.txt"));
        let r = api.delete(TEST_URL);
        r.expect("Error in deleting a pin.");

        // Deleting non-existing bookmark
        let _m2 = start_mockito_server(
            r"^/posts/delete.+fucking\.way.*$",
            200,
            r#"{"result_code":"item not found"}"#,
        );
        let r = api.delete("http://no.fucking.way")
            .expect_err("Deleted non-existing pin");
        assert_eq!("item not found".to_string(), r.cause().to_string());

        // Deleting bookmark with a malformed url
        let e = api.delete(":// bad url/#")
            .expect_err("Deleted malformed url");

        // Two ways of checking
        assert_eq!(
            &ParseError::RelativeUrlWithoutBase,
            e.root_cause().downcast_ref::<ParseError>().unwrap()
        );
        // Or
        if let Some(t) = e.cause().downcast_ref::<ParseError>() {
            match t {
                &ParseError::RelativeUrlWithoutBase => (),
                _ => panic!("Deleted a malformed url"),
            }
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
        assert_eq!(vec!["datetime", "library", "rust"], res.unwrap());

        let url = ":// bad url/#";
        let error = api.suggest_tags(url)
            .expect_err("Suggested tags for malformed url");

        assert_eq!(
            &ParseError::RelativeUrlWithoutBase,
            error.root_cause().downcast_ref::<ParseError>().unwrap()
        );
    }

    #[test]
    fn test_tag_freq() {
        let _ = env_logger::try_init();
        debug!("test_tag_freq: starting.");
        let _m1 = start_mockito_server(
            r"^/tags/get.*$",
            200,
            PathBuf::from("tests/all_tags_mockito.json"),
        );
        let api = Api::new(include_str!("api_token.txt"));
        let res = api.tags_frequency();
        let _r = res.unwrap_or_else(|e| panic!("{:?}", e));
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

        assert_eq!(57, res.unwrap_or_else(|e| panic!("{:?}", e)).len());
    }
}
