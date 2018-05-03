use env_logger;
use std::path::PathBuf;

#[cfg(test)]
use mockito::{mock, Matcher, Mock};

#[cfg(test)]
pub trait MockBodyGenerate {
    fn create_mockito_server(self, endpoint: &str, status: usize) -> Mock;
}

#[cfg(test)]
impl<'a> MockBodyGenerate for &'a str {
    fn create_mockito_server(self, endpoint: &str, status: usize) -> Mock {
        mock("GET", Matcher::Regex(endpoint.to_string()))
            .with_status(status)
            .with_header("content-type", "application/json")
            .with_body(self)
            .create()
    }
}

#[cfg(test)]
impl MockBodyGenerate for PathBuf {
    fn create_mockito_server(self, endpoint: &str, status: usize) -> Mock {
        mock("GET", Matcher::Regex(endpoint.to_string()))
            .with_status(status)
            .with_header("content-type", "application/json")
            .with_body_from_file(self.to_str().expect("can't get file name's str"))
            .create()
    }
}

#[cfg(test)]
pub fn start_mockito_server<T: MockBodyGenerate>(endpoint: &str, status: usize, body: T) -> Mock {
    let _ = env_logger::try_init();
    debug!("get_server: starting");
    body.create_mockito_server(endpoint, status)
}

#[cfg(test)]
pub fn create_mockito_servers() -> (Mock, Mock) {
    let _ = env_logger::try_init();
    debug!("create_mockito_servers: starting.");
    let m1 = mock("GET", Matcher::Regex(r"^/posts/all.*$".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body_from_file("tests/all_pins_mockito.json")
        .create();
    let m2 = mock("GET", Matcher::Regex(r"^/tags/get.*$".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body_from_file("tests/all_tags_mockito.json")
        .create();
    (m1, m2)
}
