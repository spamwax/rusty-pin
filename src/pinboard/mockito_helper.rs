use env_logger;

#[cfg(test)]
use mockito::{mock, Matcher, Mock};

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

#[cfg(test)]
pub fn create_mockito_server(endpoint: String, status: usize, body: &str) -> Mock {
    let _ = env_logger::try_init();
    debug!("create_mockito_server: starting.");
    mock("GET", Matcher::Regex(endpoint))
        .with_status(status)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create()
}

#[cfg(test)]
pub fn create_mockito_server_from_file(endpoint: String, status: usize, file: &str) -> Mock {
    let _ = env_logger::try_init();
    debug!("create_mockito_server_from_file: starting.");
    mock("GET", Matcher::Regex(endpoint))
        .with_status(status)
        .with_header("content-type", "application/json")
        .with_body_from_file(file)
        .create()
}
