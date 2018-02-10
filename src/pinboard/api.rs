use std::borrow::Cow;

use serde_json;
use reqwest;
use reqwest::IntoUrl;

use chrono::prelude::*;
use url::Url;

use env_logger;

use std::io::Read;
use std::collections::HashMap;

use super::pin::{Pin, Tag};

#[cfg(not(test))]
const BASE_URL: &'static str = "https://api.pinboard.in/v1";

#[cfg(test)]
use mockito;
#[cfg(test)]
const BASE_URL: &'static str = mockito::SERVER_URL;

#[derive(Serialize, Deserialize, Debug)]
struct ApiResult {
    result_code: String,
}

impl ApiResult {
    fn ok(self) -> Result<(), String> {
        if self.result_code == "done" {
            Ok(())
        } else {
            Err(self.result_code)
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateTime {
    #[serde(rename = "update_time")]
    datetime: DateTime<Utc>,
}

#[derive(Debug)]
pub struct Api<'a> {
    auth_token: Cow<'a, str>,
}

impl<'a> Api<'a> {
    pub fn new<S>(auth_token: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        Api {
            auth_token: auth_token.into(),
        }
    }

    fn add_auth_token<T: IntoUrl>(&self, url: T) -> Url {
        let _ = env_logger::try_init();
        info!("add_auth_token: starting.");
        Url::parse_with_params(
            url.into_url().unwrap().as_ref(),
            &[("format", "json"), ("auth_token", &self.auth_token)],
        ).unwrap()
    }

    pub fn all_pins(&self) -> Result<Vec<Pin>, String> {
        let _ = env_logger::try_init();
        info!("all_pins: starting.");
        self.get_api_response([BASE_URL, "/posts/all"].concat().as_str(), &HashMap::new())
            .and_then(|res| {
                serde_json::from_str(&res)
                    .map_err(|_| "Unrecognized response from server API: posts/all".to_owned())
            })
    }

    pub fn suggest_tags<T: IntoUrl>(&self, url: T) -> Result<Vec<String>, String> {
        let _ = env_logger::try_init();
        info!("suggest_tags: starting.");
        let mut query = HashMap::new();
        query.insert(
            "url",
            url.into_url()
                .map_err(|_| "Invalid url.".to_owned())?
                .to_string(),
        );

        self.get_api_response([BASE_URL, "/posts/suggest"].concat().as_str(), &query)
            .and_then(|res| {
                serde_json::from_str::<Vec<serde_json::Value>>(&res)
                    .map_err(|_| "Bad JSON format from server API: posts/suggest".to_owned())
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
            .ok_or_else(|| "Unrecognized response from server API: posts/suggest".to_owned())
    }

    pub fn add_url(&self, p: Pin) -> Result<(), String> {
        info!("add_url: starting.");
        let _ = env_logger::try_init();
        let mut map = HashMap::new();
        let url = p.url.into_string();

        map.insert("url", url);
        map.insert("description", p.title);
        map.insert("tags", p.tags);
        map.insert("toread", p.toread);
        map.insert("extended", p.extended.unwrap_or_default());
        map.insert("shared", p.shared);
        map.insert("replace", "yes".to_string());

        info!("Sending payload to: {}/posts/add\n\t{:?}", BASE_URL, map);
        self.get_api_response([BASE_URL, "/posts/add"].concat().as_str(), &map)
            .and_then(|res| {
                serde_json::from_str::<ApiResult>(&res)
                    .map_err(|_| "Unrecognized response from server API: posts/add".to_owned())
            })
            .and_then(|r| r.ok())
    }

    pub fn tags_frequency(&self) -> Result<Vec<Tag>, String> {
        let _ = env_logger::try_init();
        info!("tags_frequency: starting.");
        self.get_api_response([BASE_URL, "/tags/get"].concat().as_str(), &HashMap::new())
            .and_then(|res| {
                serde_json::from_str(&res)
                    .map_err(|_| "Unrecognized response from server API: tags/get".to_owned())
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

    pub fn delete<T: IntoUrl>(&self, url: T) -> Result<(), String> {
        let _ = env_logger::try_init();
        info!("delete: starting.");
        let mut map = HashMap::new();
        let url = url.into_url()
            .map_err(|_| "Invalid url.".to_owned())?
            .to_string();
        map.insert("url", url);
        self.get_api_response([BASE_URL, "/posts/delete"].concat().as_str(), &map)
            .and_then(|res| {
                serde_json::from_str(&res)
                    .map_err(|_| "Unrecognized response from server API: posts/delete".to_owned())
            })
            .and_then(|r: ApiResult| r.ok())
    }

    pub fn recent_update(&self) -> Result<DateTime<Utc>, String> {
        info!("recent_update: starting.");
        self.get_api_response(
            [BASE_URL, "/posts/update"].concat().as_str(),
            &HashMap::new(),
        ).and_then(|res| {
                serde_json::from_str(&res)
                    .map_err(|_| "Unrecognized response from server API: posts/update".to_owned())
            })
            .and_then(|date: UpdateTime| Ok(date.datetime))
    }

    fn get_api_response<T: IntoUrl>(
        &self,
        endpoint: T,
        params: &HashMap<&str, String>,
    ) -> Result<String, String> {
        let _ = env_logger::try_init();
        info!("get_api_response: starting.");
        let client = reqwest::Client::new();
        let mut api_url = self.add_auth_token(endpoint);

        for (k, v) in params {
            api_url.query_pairs_mut().append_pair(k, v);
        }

        let mut resp = client.get(api_url).send().map_err(|e| match e.get_ref() {
            Some(e) => format!("{}", e.description()),
            None => format!("Serious error making network request."),
        })?;

        // TODO: check for error status codes and return them instead of panicking.
        assert!(resp.status().is_success());

        let mut content = String::new();
        if let Err(e) = resp.read_to_string(&mut content) {
            return Err(e.to_string());
        }
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{mock, Matcher};

    use pinboard::pin::PinBuilder;

    const TEST_URL: &str = "https://githuуй.com/Здравствуйт?q=13#fragment";
    #[test]
    fn get_latest_update_time() {
        let _ = env_logger::try_init();
        info!("get_latest_update_time: starting.");
        let _m = mock("GET", Matcher::Regex(r"^/posts/update.*$".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"update_time":"2018-02-07T01:54:09Z"}"#)
            .create();
        let api = Api::new(include_str!("api_token.txt").to_string());
        let r = api.recent_update();
        assert!(r.is_ok());
        println!("{:?}", r.unwrap());
    }

    #[test]
    fn delete_a_pin() {
        let _ = env_logger::try_init();
        info!("delete_a_pin: starting.");
        add_a_url();
        let _m1 = mock("GET", Matcher::Regex(r"^/posts/delete.*$".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"result_code":"done"}"#)
            .create();
        let api = Api::new(include_str!("api_token.txt").to_string());
        let r = api.delete(TEST_URL);
        r.expect("Error in deleting a pin.");

        let _m2 = mock(
            "GET",
            Matcher::Regex(r"^/posts/delete.+fucking\.way.*$".to_string()),
        ).with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"result_code":"item not found"}"#)
            .create();
        let r = api.delete("http://no.fucking.way");
        assert_eq!(
            "item not found".to_owned(),
            r.expect_err("Deleted non-existing pin!")
        );

        let r = api.delete(":// bad url/#");
        assert_eq!(
            "Invalid url.".to_owned(),
            r.expect_err("Deleted malformed url!")
        );
    }

    #[test]
    fn add_a_url() {
        let _ = env_logger::try_init();
        info!("add_a_url: starting.");
        let _m1 = mock("GET", Matcher::Regex(r"^/posts/add.*$".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"result_code":"done"}"#)
            .create();
        let api = Api::new(include_str!("api_token.txt").to_string());
        let p = PinBuilder::new(TEST_URL, "test bookmark/pin".to_string())
            .tags("tagestan what".to_string())
            .description("russian website!".to_string())
            .shared("yes")
            .into_pin();
        let res = api.add_url(p);
        res.expect("Error in adding.");
    }

    #[test]
    fn suggest_tags() {
        let _ = env_logger::try_init();
        info!("suggest_tags: starting.");
        let _m1 = mock("GET", Matcher::Regex(r"^/posts/suggest.*$".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_file("tests/suggested_tags_mockito.json")
            .create();
        let api = Api::new(include_str!("api_token.txt").to_string());
        let url = "http://blog.com/";
        let res = api.suggest_tags(url);
        assert_eq!(vec!["datetime", "library", "rust"], res.unwrap());

        let url = ":// bad url/#";
        let res = api.suggest_tags(url);
        assert!(res.is_err());
        assert_eq!(
            "Invalid url.".to_owned(),
            res.expect_err("Getting tag suggestion for malformed url should fail.")
        );
    }

    #[test]
    fn test_tag_freq() {
        let _ = env_logger::try_init();
        info!("test_tag_freq: starting.");
        let _m1 = mock("GET", Matcher::Regex(r"^/tags/get.*$".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_file("tests/all_tags_mockito.json")
            .create();
        let api = Api::new(include_str!("api_token.txt").to_string());
        let res = api.tags_frequency();
        let _r = res.unwrap_or_else(|e| panic!("{:?}", e));
    }

    #[ignore]
    #[test]
    fn test_all_pins() {
        let _ = env_logger::try_init();
        info!("test_all_pins: starting.");
        let _m1 = mock("GET", Matcher::Regex(r"^/posts/all.*$".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_file("tests/all_pins_mockito.json")
            .create();
        let api = Api::new(include_str!("api_token.txt").to_string());
        let res = api.all_pins();

        assert_eq!(57, res.unwrap_or_else(|e| panic!("{:?}", e)).len());
    }
}
