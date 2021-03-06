// TODO: Add tests for case insensitivity searches of tags/pins
use super::*;
use std::fs;

#[cfg(feature = "bench")]
use test::Bencher;

use self::mockito_helper::create_mockito_servers;
use self::mockito_helper::start_mockito_server;
use mockito::{mock, Matcher};
use url;
use url::ParseError;

use crate::tests::rand_temp_path;

const TEST_URL: &str = "https://githuуй.com/Здравствуйт?q=13#fragment";

#[test]
fn test_cached_file_names() {
    let _ = env_logger::try_init();
    debug!("test_cached_data: starting.");
    let mut h = dirs::home_dir().unwrap();
    h.push(".cache");
    h.push("rusty-pin");
    let p: Option<PathBuf> = None;
    let c = CachedData::new(p).expect("Can't initiate 'CachedData'.");
    assert_eq!(c.cache_dir, h);

    // const TAGS_CACHE_FN: &str = "tags.cache";
    // const PINS_CACHE_FN: &str = "pins.cache";
    h.push("pins");
    h.set_extension("cache");
    assert_eq!(c.pins_cache_file, h);

    h.set_file_name("tags");
    h.set_extension("cache");
    assert_eq!(c.tags_cache_file, h);
}

#[test]
fn test_set_cache_dir() {
    let _ = env_logger::try_init();
    debug!("test_set_cache_dir: starting.");
    let mut h = dirs::home_dir().unwrap();
    let p: Option<PathBuf> = None;
    let mut c = CachedData::new(p).expect("Can't initiate 'CachedData'.");

    h.push(".cache");
    h.push("rusty-pin");
    c.set_cache_dir(&h).expect("Can't change cache path.");

    h.push("tags.cache");
    assert_eq!(c.tags_cache_file, h);

    h.set_file_name("pins.cache");
    assert_eq!(c.pins_cache_file, h);
}

#[test]
fn find_url_test() {
    let _ = env_logger::try_init();
    debug!("find_url_test: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut _home = dirs::home_dir().unwrap();
    _home.push(".cache");
    _home.push("mockito-rusty-pin");
    let cache_path = Some(_home);

    let mut pinboard =
        Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");
    pinboard.enable_fuzzy_search(false);

    let r =
        pinboard.find_url("http://blog.khubla.com/freebsd/time-machine-backups-using-freebsd-zfs");
    assert!(r.is_ok());
    let op = r.unwrap();
    assert!(op.is_some());
    let pins = op.unwrap();

    assert_eq!(1, pins.len());

    let r =
        pinboard.find_url("http://blog.khubla.com/freebsd/time-machine-backups-using-FreeBSD-zfs");
    assert!(r.is_ok());
    let op = r.unwrap();
    assert!(op.is_some());
    let pins = op.unwrap();

    assert_eq!(1, pins.len());

    let r =
        pinboard.find_url("https://blog.khubla.com/freebsd/time-machine-backups-using-FreeBSD-zfs");
    assert!(r.is_ok());
    let op = r.unwrap();
    assert!(op.is_none());
}

#[test]
fn test_search_items() {
    let _ = env_logger::try_init();
    debug!("test_search_items: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut _home = dirs::home_dir().unwrap();
    _home.push(".cache");
    _home.push("mockito-rusty-pin");
    let cache_path = Some(_home);

    let mut pinboard =
        Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");
    pinboard.enable_fuzzy_search(false);

    {
        let pins = pinboard
            .search_items("openpgp")
            .unwrap_or_else(|e| panic!(e));
        assert!(pins.is_some());
    }

    {
        // non-fuzzy search test
        let pins = pinboard
            .search_items("non-existence-tag")
            .unwrap_or_else(|e| panic!(e));
        assert!(pins.is_none());
    }
    {
        // fuzzy search test
        pinboard.enable_fuzzy_search(true);
        let pins = pinboard
            .search_items("gemhobi")
            .unwrap_or_else(|e| panic!(e));
        assert!(pins.is_some());
        assert_eq!(1, pins.unwrap().len());
    }
}

#[test]
fn list_tag_pairs() {
    use self::tag::TagFreq;
    let _ = env_logger::try_init();
    debug!("search_tag_pairs: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut _home = dirs::home_dir().unwrap();
    _home.push(".cache");
    _home.push("mockito-rusty-pin");
    let cache_path = Some(_home);

    let mut pinboard =
        Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");
    pinboard.enable_fuzzy_search(false);

    let tp = pinboard.list_tag_pairs();
    assert!(tp.is_some());
    assert_eq!(94, tp.as_ref().map(|tp| tp.len()).unwrap());
    for (idx, freq) in &[(0usize, 10usize), (3, 4), (93, 1)] {
        match tp.as_ref().unwrap()[*idx].1 {
            TagFreq::Used(x) => assert_eq!(*freq, x as usize),
            _ => panic!(
                "Wrong value for tag freq: {:?}",
                tp.as_ref().unwrap()[*idx].1
            ),
        }
    }
}

#[test]
fn search_tag_pairs() {
    let _ = env_logger::try_init();
    debug!("search_tag_pairs: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut _home = dirs::home_dir().unwrap();
    _home.push(".cache");
    _home.push("mockito-rusty-pin");
    let cache_path = Some(_home);

    let mut pinboard =
        Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");
    pinboard.enable_fuzzy_search(false);

    {
        let tags = pinboard
            .search_list_of_tags("ctags")
            .unwrap_or_else(|e| panic!(e));
        assert!(tags.is_some());
    }

    {
        // non-fuzzy search test
        let tags = pinboard
            .search_list_of_tags("non-existence-tag")
            .unwrap_or_else(|e| panic!(e));
        assert!(tags.is_none());
    }
    {
        // fuzzy search test
        pinboard.enable_fuzzy_search(true);
        let tags = pinboard
            .search_list_of_tags("non-existence-tag")
            .unwrap_or_else(|e| panic!(e));
        assert!(tags.is_none());
    }

    {
        // non-fuzzy search test
        let tags = pinboard
            .search_list_of_tags("yubikey")
            .unwrap_or_else(|e| panic!(e));
        assert!(tags.is_some());
        let tags = tags.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(TagFreq::Used(3), tags[0].1);
    }

    {
        // fuzzy search test
        pinboard.enable_fuzzy_search(true);
        let tags = pinboard
            .search_list_of_tags("mroooe")
            .unwrap_or_else(|e| panic!(e));
        assert!(tags.is_some());
        let tags = tags.unwrap();
        assert_eq!(1, tags.len());
        assert_eq!(TagFreq::Used(5), tags[0].1);
    }

    {
        // empty query non-fuzzy
        pinboard.enable_fuzzy_search(false);
        let tags = pinboard
            .search_list_of_tags("")
            .unwrap_or_else(|e| panic!(e));
        assert!(tags.is_some());
        assert_eq!(94, tags.unwrap().len());
    }

    {
        // empty query fuzzy
        pinboard.enable_fuzzy_search(true);
        let tags = pinboard
            .search_list_of_tags("")
            .unwrap_or_else(|e| panic!(e));
        assert!(tags.is_some());
        assert_eq!(94, tags.unwrap().len());
    }
}

#[test]
fn list_tags() {
    let _ = env_logger::try_init();
    debug!("list_tags: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut _home = dirs::home_dir().unwrap();
    _home.push(".cache");
    _home.push("mockito-rusty-pin");
    let _ = fs::remove_file(&_home);
    let cache_path = Some(_home);

    let pinboard =
        Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");
    assert!(pinboard.list_tag_pairs().is_some());
}

#[test]
fn list_bookmarks() {
    let _ = env_logger::try_init();
    debug!("list_bookmarks: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut _home = dirs::home_dir().expect("Can't find home dir");
    _home.push(".cache");
    _home.push("mockito-rusty-pin");
    let cache_path = Some(_home);

    let pinboard =
        Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");
    assert!(pinboard.list_bookmarks().is_some());
}

#[test]
fn add_pin_test() {
    let _ = env_logger::try_init();
    debug!("delete_a_pin: starting.");
    let mut _home = dirs::home_dir().expect("Can't find home dir");
    _home.push(".cache");
    _home.push("mockito-rusty-pin");
    let cache_path = Some(_home);
    let pinboard =
        Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");

    {
        // add a good url
        let _m1 = start_mockito_server(r"^/posts/add.*$", 200, r#"{"result_code":"done"}"#);
        let p = PinBuilder::new(TEST_URL, "test bookmark/pin")
            .tags("tagestan what")
            .description("russian website!")
            .shared("yes")
            .into_pin();
        assert_eq!(true, pinboard.add_pin(p).is_ok());
    }
    {
        // add a bad url
        let _m1 = start_mockito_server(r"^/posts/add.+bad_url*$", 200, r#"{"result_code":"done"}"#);
        let p = PinBuilder::new(":/ bad_url", "test bookmark/pin")
            .tags("tagestan what")
            .description("russian website!")
            .shared("yes")
            .into_pin();
        let r = pinboard
            .add_pin(p)
            .expect_err("Should return parse error for malformed url");
        assert_eq!(
            &ParseError::RelativeUrlWithoutBase,
            r.find_root_cause().downcast_ref::<ParseError>().unwrap()
        );
    }
}

#[test]
fn delete_test() {
    let _ = env_logger::try_init();
    debug!("delete_a_pin: starting.");
    // let (_m1, _m2) = create_mockito_servers();
    let mut _home = dirs::home_dir().expect("Can't find home dir");
    _home.push(".cache");
    _home.push("mockito-rusty-pin");
    let cache_path = Some(_home);
    let pinboard =
        Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");

    {
        let _m1 = start_mockito_server(
            r"^/posts/delete.+good\.url.*$",
            200,
            r#"{"result_code":"done"}"#,
        );
        let _ = pinboard
            .delete("http://www.good.url.com/#")
            .expect("Should succeed deleting a malformed url");
    }

    {
        let _m1 = start_mockito_server(
            r"^/posts/delete.+bad_url.*$",
            200,
            r#"{"result_code":"item not found"}"#,
        );
        let e = pinboard
            .delete(":// bad_url/")
            .expect_err("Should not succeed deleting a malformed url");
        assert_eq!("item not found", e.as_fail().to_string());
    }

    // println!("e--> {:?}", e);
    // let e1 = e.find_root_cause().downcast_ref::<ParseError>();
    // println!("e1--> {:?}", e1);
    // assert!(e1.is_some());

    // Original error is of type reqwest::Error but returned as Fail
    // so we need to do double downcast.
    // First from Fail to reqwest::Error then to url::Error
    // let e1 = e.find_root_cause().downcast_ref::<reqwest::Error>();
    // println!("e1--> {:?}", e1);
    // assert!(e1.is_some());
    // let e2 = e1.unwrap().get_ref();
    // assert!(e2.is_some());
    // let e3 = e2.unwrap().downcast_ref::<url::ParseError>();
    // assert!(e3.is_some());
    // assert_eq!(&ParseError::RelativeUrlWithoutBase, e3.unwrap());
}
#[test]
fn popular_tags() {
    let _ = env_logger::try_init();
    debug!("popular_tags: starting.");
    let _m1 = mock("GET", Matcher::Regex(r"^/posts/suggest.*$".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"popular":["datetime","library","rust"]},{"recommended":["datetime","library","programming","rust"]}]"#)
            .create();
    let mut _home = dirs::home_dir().expect("Can't get home_dir");
    _home.push(".cache");
    _home.push("mockito-rusty-pin");
    let cache_path = Some(_home);

    let pinboard =
        Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");
    let tags = pinboard.popular_tags("https://docs.rs/chrono/0.4.0/chrono");
    assert!(tags.is_ok());
    let tags = tags.expect("impossible");
    assert!(tags.len() >= 2);

    // Test invalid URL
    let url = ":// bad url/#";
    let error = pinboard
        .popular_tags(url)
        .expect_err("Suggested tags for malformed url");
    assert_eq!(
        &url::ParseError::RelativeUrlWithoutBase,
        error
            .find_root_cause()
            .downcast_ref::<url::ParseError>()
            .unwrap()
    );
    if let Some(t) = error.as_fail().downcast_ref::<ParseError>() {
        match t {
            ParseError::RelativeUrlWithoutBase => (),
            _ => panic!("Deleted a malformed url"),
        }
    } else {
        panic!("Should have received a ParseError");
    }
}

#[test]
fn test_cached_pins_tags() {
    let _ = env_logger::try_init();
    create_mockito_servers();
    let (_m1, _m2) = create_mockito_servers();
    let mut _home = rand_temp_path();
    _home.push("mockito-rusty-pin");
    let cache_path = Some(_home);

    let mut pinboard =
        Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");
    {
        pinboard.enable_fuzzy_search(false);
        let queries = ["yubikey"];
        let fields = vec![
            SearchType::TitleOnly,
            SearchType::TagOnly,
            SearchType::DescriptionOnly,
        ];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert!(pins.is_some());
        assert_eq!(3, pins.unwrap().len());

        let queries = ["Yubikey"];
        let fields = vec![SearchType::TagOnly];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert!(pins.is_some());
        assert_eq!(3, pins.unwrap().len());
    }
}

#[test]
fn test_issue7() {
    let _ = env_logger::try_init();
    let mut _home = rand_temp_path();
    _home.push("mockito-rusty-pin");

    let cache_path = Some(_home);
    debug!("create_mockito_servers: starting.");
    let _m1 = mock("GET", Matcher::Regex(r"^/posts/all.*$".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body_from_file("tests/alfred-pinboard-rs-issue7-bookmarks.json")
        .create();
    let _m2 = mock("GET", Matcher::Regex(r"^/tags/get.*$".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body_from_file("tests/alfred-pinboard-rs-issue7-tags.json")
        .create();
    let mut pinboard =
        Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");
    // Find pins that have all keywords almost anywhere
    {
        pinboard.enable_fuzzy_search(false);
        let queries = ["iTerm"];
        let fields = vec![
            SearchType::TitleOnly,
            SearchType::TagOnly,
            SearchType::DescriptionOnly,
        ];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert!(pins.is_some());
        assert_eq!(2, pins.unwrap().len());
        {
            let queries = ["iterm"];
            let fields = vec![SearchType::TitleOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
            assert_eq!(1, pins.unwrap().len());
            let queries = ["iTerm"];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
            assert_eq!(1, pins.unwrap().len());
        }
        {
            let queries = ["iterm"];
            let fields = vec![SearchType::TagOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
            assert_eq!(2, pins.unwrap().len());
            let queries = ["iTerm"];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
            assert_eq!(2, pins.unwrap().len());
        }
        {
            let queries = ["iterm"];
            let fields = vec![SearchType::DescriptionOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_none());
            let queries = ["iTerm"];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_none());
        }
        {
            let queries = ["iterm"];
            let fields = vec![SearchType::UrlOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
            assert_eq!(1, pins.unwrap().len());
            let queries = ["iTerm"];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
            assert_eq!(1, pins.unwrap().len());
        }
        {
            let queries = ["iterm2"];
            let fields = vec![SearchType::UrlOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
            assert_eq!(1, pins.unwrap().len());
            let queries = ["iTerm2"];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
            assert_eq!(1, pins.unwrap().len());
        }
        {
            let queries = ["iterm2"];
            let fields = vec![SearchType::TagOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_none());
            let queries = ["iTerm2"];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_none());
        }
        {
            let queries = ["iterm2"];
            let fields = vec![SearchType::TitleOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
            assert_eq!(1, pins.unwrap().len());
            let queries = ["iTerm2"];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!(e));
            assert!(pins.is_some());
            assert_eq!(1, pins.unwrap().len());
        }
    }
    {
        pinboard.enable_fuzzy_search(false);
        let queries = ["homebrew"];
        let fields = vec![
            SearchType::TitleOnly,
            SearchType::TagOnly,
            SearchType::DescriptionOnly,
        ];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert!(pins.is_some());
        assert_eq!(1, pins.unwrap().len());
        let queries = ["Homebrew"];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert!(pins.is_some());
        assert_eq!(1, pins.unwrap().len());
        let queries = ["oh-my-zsh"];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert!(pins.is_some());
        assert_eq!(1, pins.unwrap().len());
    }
}

#[test]
fn search_multi_query_multi_field() {
    let _ = env_logger::try_init();
    debug!("search_multi_query_multi_field: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut _home = rand_temp_path();
    _home.push(".cache");
    _home.push("mockito-rusty-pin");
    let cache_path = Some(_home);

    let mut pinboard =
        Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");
    // Find pins that have all keywords almost anywhere
    {
        pinboard.enable_fuzzy_search(false);
        let queries = ["eagle", "design", "fun"];
        let fields = vec![
            SearchType::TitleOnly,
            SearchType::TagOnly,
            SearchType::DescriptionOnly,
        ];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert!(pins.is_some());

        // Run same query, this time with Vec<String> instead of Vec<&str>
        let queries = vec!["eagle", "design", "fun"];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert!(pins.is_some());
    }

    // Find pins that have all keywords only in Title
    {
        let fields = vec![SearchType::TitleOnly];
        let queries = ["rust", "python"];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert!(pins.is_none());
    }

    // Find pins that have all keywords only in Url
    {
        let queries = ["pixlas"];
        let fields = vec![SearchType::UrlOnly];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert_eq!(1, pins.as_ref().unwrap().len());
    }

    // Fuzzy search
    {
        pinboard.enable_fuzzy_search(true);
        let queries = ["rust", "strange", "cross", "readme", "hint"];
        let fields = vec![
            SearchType::TitleOnly,
            SearchType::TagOnly,
            SearchType::DescriptionOnly,
            SearchType::UrlOnly,
        ];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert_eq!(3, pins.as_ref().unwrap().len());
    }

    // Fuzzy search unicode
    {
        pinboard.enable_fuzzy_search(true);
        let queries = ["\u{0622}\u{0645}\u{0648}\u{0632}\u{0634}\u{06cc}"]; // آموزشی
        let fields = vec![
            SearchType::TitleOnly,
            SearchType::TagOnly,
            SearchType::DescriptionOnly,
            SearchType::UrlOnly,
        ];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert_eq!(1, pins.as_ref().unwrap().len());
    }

    // Fuzzy search unicode
    {
        pinboard.enable_fuzzy_search(true);
        // let queries = ["世"];
        let queries = ["\u{4e16}"];
        let fields = vec![
            SearchType::TitleOnly,
            SearchType::TagOnly,
            SearchType::DescriptionOnly,
            SearchType::UrlOnly,
        ];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert_eq!(3, pins.as_ref().unwrap().len());
    }

    // Tag-only search
    {
        pinboard.enable_fuzzy_search(false);
        let queries = ["rust"];
        let fields = vec![SearchType::TagOnly];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert_eq!(10, pins.as_ref().unwrap().len());

        let queries = ["yubikey"];
        let fields = vec![SearchType::TagOnly];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert_eq!(3, pins.as_ref().unwrap().len());

        let queries = ["YubiKey"];
        let fields = vec![SearchType::TagOnly];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert_eq!(3, pins.as_ref().unwrap().len());

        let queries = ["keyboard", "hacks"];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert!(pins.is_some());
        assert_eq!(1, pins.as_ref().unwrap().len());
    }

    // Tag-only search with fuzzy search
    {
        pinboard.enable_fuzzy_search(true);
        let queries = ["backup"];
        let fields = vec![SearchType::TagOnly];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert_eq!(2, pins.as_ref().unwrap().len());
    }

    // title+url search non-fuzzy
    {
        pinboard.enable_fuzzy_search(false);
        let queries = ["世", "macos"];
        let fields = vec![SearchType::TitleOnly, SearchType::UrlOnly];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert_eq!(1, pins.as_ref().unwrap().len());
    }

    // empty search query
    {
        pinboard.enable_fuzzy_search(false);
        let queries = [""];
        let fields = vec![SearchType::TitleOnly, SearchType::UrlOnly];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!(e));
        assert!(pins.is_some());
    }
}

#[test]
fn serde_update_cache() {
    let _ = env_logger::try_init();
    debug!("serde_update_cache: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut _home = dirs::home_dir().unwrap();
    _home.push(".cache");
    _home.push("mockito-rusty-pin");
    let cache_path = Some(_home.clone());

    // First remove all folders to force a full update
    fs::remove_dir_all(_home).expect("Can't remove dir to prepare the test");

    let p = Pinboard::new(include_str!("api_token.txt"), cache_path);
    let mut pinboard = p.unwrap_or_else(|e| panic!("{:?}", e));

    // Get all pins directly from Pinboard.in (no caching)
    let fresh_pins = pinboard.api.all_pins().expect("impossilbe?");

    pinboard.update_cache().expect("Couldn't update the cache");

    let cached_pins = pinboard.cached_data.pins.unwrap();
    assert_eq!(fresh_pins.len(), cached_pins.len());

    for idx in 0..fresh_pins.len() {
        info!("serde_update_cache: Checking pin[{}]", idx);
        let found = cached_pins
            .iter()
            .find(|&p| &p.pin.url == &fresh_pins[idx].url);
        assert!(found.is_some(), "{:?}", fresh_pins[idx]);
        let cached_pin = found.unwrap();
        // Title
        assert_eq!(fresh_pins[idx as usize].title, cached_pin.pin.title);
        assert_eq!(
            fresh_pins[idx as usize].title.to_lowercase(),
            cached_pin.title_lowered
        );
        // Url
        assert_eq!(fresh_pins[idx as usize].url, cached_pin.pin.url);
        // tags
        assert_eq!(fresh_pins[idx as usize].tags, cached_pin.pin.tags);
        assert_eq!(
            fresh_pins[idx as usize].tags.to_lowercase(),
            cached_pin.tag_list.join(" ")
        );
        // shared
        assert_eq!(
            fresh_pins[idx as usize].shared.to_lowercase(),
            cached_pin.pin.shared
        );
        // toread
        assert_eq!(
            fresh_pins[idx as usize].toread.to_lowercase(),
            cached_pin.pin.toread
        );
        // time
        assert_eq!(fresh_pins[idx as usize].time, cached_pin.pin.time);

        // extended
        if fresh_pins[idx as usize].extended.is_some() {
            assert!(cached_pin.pin.extended.is_some());
            assert_eq!(
                fresh_pins[idx as usize].extended.as_ref().unwrap(),
                cached_pin.pin.extended.as_ref().unwrap().as_ref()
            );
            assert_eq!(
                fresh_pins[idx as usize]
                    .extended
                    .as_ref()
                    .unwrap()
                    .to_lowercase(),
                cached_pin.extended_lowered.as_ref().unwrap().as_ref()
            )
        } else {
            assert!(cached_pin.pin.extended.is_none());
        }
    }
}

// I am not sure why I wrote this test as it is kind of similar to serde_update_cache
#[test]
fn test_update_cache() {
    let _ = env_logger::try_init();
    debug!("test_update_cache: starting.");

    const IDX: usize = 25;

    let (_m1, _m2) = create_mockito_servers();
    let mut _home = dirs::home_dir().unwrap();
    _home.push(".cache");
    _home.push("mockito-rusty-pin");

    let cache_path = Some(_home.clone());

    debug!("Running first update_cache");

    // First remove all folders to force a full update
    fs::remove_dir_all(_home).expect("Can't remove dir to prepare the test");

    // Pinboard::new() will call update_cache since we remove the cache folder.
    let pb = Pinboard::new(include_str!("api_token.txt"), cache_path);
    let mut pinboard = match pb {
        Ok(v) => v,
        Err(e) => panic!("{:?}", e),
    };
    let pins = match pinboard.cached_data.pins.take() {
        Some(v) => v,
        None => panic!("No pins found in cache!"),
    };
    let tags = match pinboard.cached_data.tags.take() {
        Some(v) => v,
        None => panic!("No tags found in cache!"),
    };
    assert!(pins.len() > IDX);
    assert!(tags.len() > IDX);

    debug!("Running second update_cache");
    pinboard
        .cached_data
        .update_cache(&pinboard.api)
        .unwrap_or_else(|e| panic!(e));
    pinboard
        .cached_data
        .load_cache_data_from_file()
        .unwrap_or_else(|e| panic!(e));
    assert!(pinboard.cached_data.cache_ok());

    assert!(pinboard.cached_data.pins.is_some());
    debug!(
        "{:?}\n\n{:?}\n\n",
        pins[IDX],
        pinboard.cached_data.pins.as_ref().unwrap()[IDX]
    );
    assert_eq!(pins[IDX], pinboard.cached_data.pins.as_ref().unwrap()[IDX]);
    assert_eq!(
        pins.len(),
        pinboard.cached_data.pins.as_ref().unwrap().len()
    );

    assert!(pinboard.cached_data.tags.is_some());
    debug!(
        "{:?}\n{:?}",
        tags[IDX],
        pinboard.cached_data.tags.as_ref().unwrap()[IDX].tag
    );
    assert_eq!(
        tags.len(),
        pinboard.cached_data.tags.as_ref().unwrap().len()
    );
    assert_eq!(tags[IDX], pinboard.cached_data.tags.as_ref().unwrap()[IDX]);
}

#[cfg(feature = "bench")]
#[bench]
fn bench_search_items_openpgp(b: &mut Bencher) {
    let _ = env_logger::try_init();
    debug!("bench_search_items_non_fuzzy: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut _home = dirs::home_dir().unwrap();
    _home.push(".cache");
    _home.push("mockito-rusty-pin");
    let cache_path = Some(_home);

    let mut pinboard =
        Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");
    pinboard.enable_fuzzy_search(false);
    pinboard.enable_tag_only_search(false);
    let query = "openpgp";
    b.iter(|| {
        let _ = pinboard.search_items(query).unwrap_or_else(|e| panic!(e));
    })
}

#[cfg(feature = "bench")]
#[bench]
fn bench_search_openpgp(b: &mut Bencher) {
    let _ = env_logger::try_init();
    debug!("bench_search_openpgp: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut _home = dirs::home_dir().unwrap();
    _home.push(".cache");
    _home.push("mockito-rusty-pin");
    let cache_path = Some(_home);

    let mut pinboard =
        Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");
    pinboard.enable_fuzzy_search(false);
    pinboard.enable_tag_only_search(false);
    let queries = ["openpgp"];
    let fields = vec![
        SearchType::TitleOnly,
        SearchType::TagOnly,
        SearchType::UrlOnly,
        SearchType::DescriptionOnly,
    ];
    b.iter(|| {
        let _pins = pinboard
            .search(&queries, fields.as_slice())
            .unwrap_or_else(|e| panic!(e));
    });
}

#[cfg(feature = "bench")]
#[bench]
fn bench_search_non_fuzzy(b: &mut Bencher) {
    let _ = env_logger::try_init();
    debug!("bench_search_non_fuzzy: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut _home = dirs::home_dir().unwrap();
    _home.push(".cache");
    _home.push("mockito-rusty-pin");
    let cache_path = Some(_home);

    let mut pinboard =
        Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");
    pinboard.enable_fuzzy_search(false);
    let queries = ["zfs", "fr"];
    let fields = vec![];
    b.iter(|| {
        let _pins = pinboard
            .search(&queries, fields.as_slice())
            .unwrap_or_else(|e| panic!(e));
    });
}

#[cfg(feature = "bench")]
#[bench]
fn bench_search_fuzzy(b: &mut Bencher) {
    let _ = env_logger::try_init();
    debug!("bench_search_fuzzy: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut _home = dirs::home_dir().unwrap();
    _home.push(".cache");
    _home.push("mockito-rusty-pin");
    let cache_path = Some(_home);

    let mut pinboard =
        Pinboard::new(include_str!("api_token.txt"), cache_path).expect("Can't setup Pinboard");
    pinboard.enable_fuzzy_search(true);
    let queries = ["zfs", "fr"];
    let fields = vec![];
    b.iter(|| {
        let _pins = pinboard
            .search(&queries, fields.as_slice())
            .unwrap_or_else(|e| panic!(e));
    });
}
