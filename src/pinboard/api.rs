const TOKEN: &'static str = include_str!("auth_token.txt");

use serde_json;
use reqwest;

use chrono::prelude::*;
use url::Url;

fn add_auth_token(url: &str) -> Url {
    Url::parse_with_params(url, &[("format", "json"), ("auth_token", TOKEN)]).unwrap()
}

pub fn recent_update() -> Result<DateTime<Utc>, String> {
    use std::io::Read;

    #[derive(Serialize, Deserialize, Debug)]
    struct UpdateTime {
        #[serde(rename = "update_time")]
        datetime: DateTime<Utc>,
    }

    let res = reqwest::get(add_auth_token("https://api.pinboard.in/v1/posts/update"));
    let mut resp = match res {
        Ok(resp) => resp,
        Err(e) => return Err(e.to_string()),
    };

    //TODO: check for error status codes and return them instead of panicking.
    assert!(resp.status().is_success());

    let mut content = String::new();
    if let Err(e) = resp.read_to_string(&mut content) {
        return Err(e.to_string());
    }

    let date: Result<UpdateTime, _> = serde_json::from_str(&content);
    match date {
        Ok(date) => Ok(date.datetime),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_latest_update_time() {
        let r = recent_update();
        assert!(r.is_ok());
        println!("{:?}", r.unwrap());
    }
}
