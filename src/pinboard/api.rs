const TOKEN: &'static str = include_str!("auth_token.txt");

use serde_json;
use reqwest;
use reqwest::IntoUrl;

use chrono::prelude::*;
use url::Url;

use std::io::Read;
use std::collections::HashMap;

use super::{Pin, Tag};

#[derive(Serialize, Deserialize, Debug)]
struct ApiResult {
    result_code: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateTime {
    #[serde(rename = "update_time")]
    datetime: DateTime<Utc>,
}

#[derive(Debug)]
pub struct Api {
    auth_token: String,
}

impl Api {
    pub fn new(auth_token: String) -> Self {
        Api { auth_token }
    }

    fn add_auth_token<T: IntoUrl>(&self, url: T) -> Url {
        Url::parse_with_params(
            url.into_url().unwrap().as_ref(),
            &[("format", "json"), ("auth_token", &self.auth_token)],
        ).unwrap()
    }

    pub fn all_pins(&self) -> Result<Vec<Pin>, String> {
        let res = self.get_api_response(
            "https://api.pinboard.in/v1/posts/all",
            HashMap::new(),
        )?;
        let res: Result<Vec<Pin>, _> = serde_json::from_str(&res);

        if let Err(e) = res {
            Err(format!("{:?}", e))
        } else {
            Ok(res.unwrap())
        }
    }

    pub fn suggest_tags<T: IntoUrl>(self, url: T) -> Result<Vec<String>, String> {
        let mut query = HashMap::new();
        query.insert("url", url.into_url().unwrap().to_string());

        let res = self.get_api_response(
            "https://api.pinboard.in/v1/posts/suggest",
            query,
        )?;
        let res: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&res);

        if let Err(e) = res {
            return Err(format!("Unrecognized response from server: {:?}", e));
        }
        for item in res.unwrap() {
            if !item["popular"].is_null() {
                return Ok(
                    item["popular"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_str().unwrap().to_string())
                        .collect::<Vec<String>>(),
                );
            }
        }
        Err("Unrecognized response from server".to_string())
    }

    pub fn add_url(self, p: Pin) -> Result<(), String> {
        let mut map = HashMap::new();
        let url = p.url.into_string();

        map.insert("url", url.clone());
        map.insert("description", p.title);
        map.insert("toread", p.toread);
        map.insert("extended", p.extended.unwrap_or_default());
        map.insert("shared", p.shared);
        map.insert("replace", "yes".to_string());

        let res = self.get_api_response(
            "https://api.pinboard.in/v1/posts/add",
            map,
        )?;
        let res: Result<ApiResult, _> = serde_json::from_str(&res);

        match res {
            Ok(ref r) if r.result_code == "done" => Ok(()),
            Ok(r) => Err(r.result_code),
            Err(e) => Err(format!("Unrecognized response from server: {:?}", e)),
        }
    }

    pub fn tags_frequency(&self) -> Result<Vec<Tag>, String> {
        let res = self.get_api_response(
            "https://api.pinboard.in/v1/tags/get",
            HashMap::new(),
        )?;

        let res: Result<HashMap<String, String>, _> = serde_json::from_str(&res);
        if let Err(e) = res {
            Err(format!("Unrecognized server response: {:?}", e))
        } else {
            Ok(
                res.unwrap()
                    .into_iter()
                    .map(|(k, v)| {
                        let freq = v.parse::<usize>().unwrap_or_default();
                        Tag(k, freq)
                    })
                    .collect(),
            )
        }
    }

    pub fn delete<T: IntoUrl>(self, url: T) -> Result<(), String> {
        let mut map = HashMap::new();
        let url = url.into_url().unwrap().to_string();
        map.insert("url", url.clone());
        let resp = self.get_api_response(
            "https://api.pinboard.in/v1/posts/delete",
            map,
        )?;

        let resp: Result<ApiResult, _> = serde_json::from_str(&resp);
        match resp {
            Ok(ref r) if r.result_code == "done" => Ok(()),
            Ok(r) => Err(r.result_code),
            Err(e) => Err(format!("Unrecognized response from server: {:?}", e)),
        }
    }

    pub fn recent_update(self) -> Result<DateTime<Utc>, String> {
        let content = self.get_api_response(
            "https://api.pinboard.in/v1/posts/update",
            HashMap::new(),
        )?;
        let date: Result<UpdateTime, _> = serde_json::from_str(&content);
        match date {
            Ok(date) => Ok(date.datetime),
            Err(e) => Err(e.to_string()),
        }
    }

    fn get_api_response<T: IntoUrl>(
        &self,
        endpoint: T,
        params: HashMap<&str, String>,
    ) -> Result<String, String> {
        let client = reqwest::Client::new();
        let mut api_url = self.add_auth_token(endpoint);

        for (k, v) in &params {
            api_url.query_pairs_mut().append_pair(k, v);
        }
        let res = client.get(api_url).send();

        let mut resp = match res {
            Ok(msg) => msg,
            Err(e) => return Err(e.to_string()),
        };

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

    use pinboard::PinBuilder;

    #[test]
    fn get_latest_update_time() {
        let api = Api::new(include_str!("auth_token.txt").to_string());
        let r = api.recent_update();
        assert!(r.is_ok());
        println!("{:?}", r.unwrap());
    }

    #[test]
    fn delete_a_pin() {
        add_a_url();
        let api = Api::new(include_str!("auth_token.txt").to_string());
        let r = api.delete("https://githuуй.com/Здравствуйт?q=13#fragment");
        r.expect("Error in deleting a pin.");
    }

    #[test]
    fn add_a_url() {
        let api = Api::new(include_str!("auth_token.txt").to_string());
        let p = PinBuilder::new(
            "https://githuуй.com/Здравствуйт?q=13#fragment",
            "test bookmark/pin".to_string(),
        ).into_pin();
        let res = api.add_url(p);
        res.expect("Error in adding.");
    }

    #[test]
    fn suggest_tags() {
        let api = Api::new(include_str!("auth_token.txt").to_string());
        let url = "http://blog.com/";
        let res = api.suggest_tags(url);
        println!("{:?}", res);
        assert_eq!(res.unwrap(), vec!["blog", "blogging", "free", "hosting"]);
    }

    #[test]
    fn test_tag_freq() {
        let api = Api::new(include_str!("auth_token.txt").to_string());
        let res = api.tags_frequency();
        assert!(res.is_ok());
    }

    #[ignore]
    #[test]
    fn test_all_pins() {
        let api = Api::new(include_str!("auth_token.txt").to_string());
        let res = api.all_pins();

        if res.is_err() {
            println!("{:?}", res)
        } else {
            println!("Got {} pins!!!", res.unwrap().len());
        }
    }
}
