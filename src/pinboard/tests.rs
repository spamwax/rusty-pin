#![allow(clippy::unicode_not_nfc)]
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
fn find_tag_test() {
    let _ = env_logger::try_init();
    debug!("find_url_test: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut myhome = dirs::home_dir().unwrap();
    myhome.push(".cache");
    myhome.push("mockito-rusty-pin");
    let cache_path = Some(myhome);

    let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path)
        .expect("Can't setup Pinboard")
        .pinboard;
    pinboard.enable_fuzzy_search(false);

    let r = pinboard.find_tag("timemachine");
    assert!(r.is_ok());
    let op = r.unwrap();
    assert!(op.is_some());
    let pins = op.unwrap();
    assert_eq!(3, pins.len());

    // Case insensitive search
    let r = pinboard.find_tag("TimeMachine");
    assert!(r.is_ok());
    let op = r.unwrap();
    assert!(op.is_some());
    let pins = op.unwrap();
    assert_eq!(3, pins.len());

    let r = pinboard.find_tag("hacks");
    assert!(r.is_ok());
    let op = r.unwrap();
    assert!(op.is_some());
    let pins = op.unwrap();
    assert_eq!(1, pins.len());

    // Should find exact tags only
    let r = pinboard.find_tag("hack");
    assert!(r.is_ok());
    let op = r.unwrap();
    assert!(op.is_none());
}

#[test]
fn find_url_test() {
    let _ = env_logger::try_init();
    debug!("find_url_test: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut myhome = dirs::home_dir().unwrap();
    myhome.push(".cache");
    myhome.push("mockito-rusty-pin");
    let cache_path = Some(myhome);

    let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path)
        .expect("Can't setup Pinboard")
        .pinboard;
    pinboard.enable_fuzzy_search(false);

    let r =
        pinboard.find_url("http://blog.khubla.com/freebsd/time-machine-backups-using-freebsd-zfs");
    assert!(r.is_ok());
    let op = r.unwrap();
    assert!(op.is_some());
    let pins = op.unwrap();

    assert_eq!(1, pins.len());

    // find_url is case insensitive.
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
    let mut myhome = dirs::home_dir().unwrap();
    myhome.push(".cache");
    myhome.push("mockito-rusty-pin");
    let cache_path = Some(myhome);

    let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path)
        .expect("Can't setup Pinboard")
        .pinboard;
    pinboard.enable_fuzzy_search(false);

    {
        let pins = pinboard
            .search_items("openpgp")
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(pins.is_some());
    }

    {
        // non-fuzzy search test
        let pins = pinboard
            .search_items("non-existence-tag")
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(pins.is_none());
    }
    {
        // fuzzy search test
        pinboard.enable_fuzzy_search(true);
        let pins = pinboard
            .search_items("gemhobi")
            .unwrap_or_else(|e| panic!("{}", e));
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
    let mut myhome = dirs::home_dir().unwrap();
    myhome.push(".cache");
    myhome.push("mockito-rusty-pin");
    let cache_path = Some(myhome);

    let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path)
        .expect("Can't setup Pinboard")
        .pinboard;
    pinboard.enable_fuzzy_search(false);

    let tp = pinboard.list_tag_pairs();
    assert!(tp.is_some());
    assert_eq!(94, tp.as_ref().map(Vec::len).unwrap());
    for (idx, freq) in &[(0usize, 10usize), (3, 4), (93, 1)] {
        match tp.as_ref().unwrap()[*idx].1 {
            TagFreq::Used(x) => assert_eq!(*freq, x),
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
    let mut myhome = dirs::home_dir().unwrap();
    myhome.push(".cache");
    myhome.push("mockito-rusty-pin");
    let cache_path = Some(myhome);

    let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path)
        .expect("Can't setup Pinboard")
        .pinboard;
    pinboard.enable_fuzzy_search(false);

    {
        let tags = pinboard
            .search_list_of_tags("ctags")
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(tags.is_some());
    }

    {
        // non-fuzzy search test
        let tags = pinboard
            .search_list_of_tags("non-existence-tag")
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(tags.is_none());
    }
    {
        // fuzzy search test
        pinboard.enable_fuzzy_search(true);
        let tags = pinboard
            .search_list_of_tags("non-existence-tag")
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(tags.is_none());
    }

    {
        // non-fuzzy search test
        let tags = pinboard
            .search_list_of_tags("yubikey")
            .unwrap_or_else(|e| panic!("{}", e));
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
            .unwrap_or_else(|e| panic!("{}", e));
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
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(tags.is_some());
        assert_eq!(94, tags.unwrap().len());
    }

    {
        // empty query fuzzy
        pinboard.enable_fuzzy_search(true);
        let tags = pinboard
            .search_list_of_tags("")
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(tags.is_some());
        assert_eq!(94, tags.unwrap().len());
    }
}

#[test]
fn list_tags() {
    let _ = env_logger::try_init();
    debug!("list_tags: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut myhome = dirs::home_dir().unwrap();
    myhome.push(".cache");
    myhome.push("mockito-rusty-pin");
    let _r = fs::remove_file(&myhome);
    let cache_path = Some(myhome);

    let pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path)
        .expect("Can't setup Pinboard")
        .pinboard;
    assert!(pinboard.list_tag_pairs().is_some());
}

#[test]
fn list_bookmarks() {
    let _ = env_logger::try_init();
    debug!("list_bookmarks: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut myhome = dirs::home_dir().expect("Can't find home dir");
    myhome.push(".cache");
    myhome.push("mockito-rusty-pin");
    let cache_path = Some(myhome);

    let pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path)
        .expect("Can't setup Pinboard")
        .pinboard;
    assert!(pinboard.list_bookmarks().is_some());
}

#[test]
fn add_pin_test() {
    let _ = env_logger::try_init();
    debug!("delete_a_pin: starting.");
    let mut myhome = dirs::home_dir().expect("Can't find home dir");
    myhome.push(".cache");
    myhome.push("mockito-rusty-pin");
    let cache_path = Some(myhome);
    let pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path)
        .expect("Can't setup Pinboard")
        .pinboard;

    {
        // add a good url
        let _m1 = start_mockito_server(r"^/posts/add.*$", 200, r#"{"result_code":"done"}"#);
        let p = PinBuilder::new(TEST_URL, "test bookmark/pin")
            .tags("tagestan what")
            .description("russian website!")
            .shared("yes")
            .into_pin();
        assert!(pinboard.add_pin(p).is_ok());
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
            r.downcast_ref::<ParseError>().unwrap()
        );
    }
}

#[test]
fn delete_test() {
    let _ = env_logger::try_init();
    debug!("delete_a_pin: starting.");
    // let (_m1, _m2) = create_mockito_servers();
    let mut myhome = dirs::home_dir().expect("Can't find home dir");
    myhome.push(".cache");
    myhome.push("mockito-rusty-pin");
    let cache_path = Some(myhome);
    let pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path)
        .expect("Can't setup Pinboard")
        .pinboard;

    {
        let _m1 = start_mockito_server(
            r"^/posts/delete.+good\.url.*$",
            200,
            r#"{"result_code":"done"}"#,
        );
        pinboard
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
        assert_eq!("item not found", e.to_string());
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
    let mut myhome = dirs::home_dir().expect("Can't get home_dir");
    myhome.push(".cache");
    myhome.push("mockito-rusty-pin");
    let cache_path = Some(myhome);

    let pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path)
        .expect("Can't setup Pinboard")
        .pinboard;
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
        error.downcast_ref::<url::ParseError>().unwrap()
    );
    if let Some(t) = error.downcast_ref::<ParseError>() {
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
    let mut myhome = rand_temp_path();
    myhome.push("mockito-rusty-pin");
    let cache_path = Some(myhome);

    let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path)
        .expect("Can't setup Pinboard")
        .pinboard;
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
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(pins.is_some());
        assert_eq!(3, pins.unwrap().len());

        let queries = ["Yubikey"];
        let fields = vec![SearchType::TagOnly];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(pins.is_some());
        assert_eq!(3, pins.unwrap().len());
    }
}

#[test]
fn test_special_char_glob() {
    let _ = env_logger::try_init();
    let mut myhome = rand_temp_path();
    myhome.push("mockito-rusty-pin");

    let cache_path = Some(myhome);
    debug!("create_mockito_servers: starting.");
    let _m1 = mock("GET", Matcher::Regex(r"^/posts/all.*$".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body_from_file("tests/globmark.json")
        .create();
    let _m2 = mock("GET", Matcher::Regex(r"^/tags/get.*$".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body_from_file("tests/globtag.json")
        .create();
    let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path)
        .expect("Can't setup Pinboard")
        .pinboard;
    {
        let fields = vec![SearchType::TitleOnly];
        pinboard.enable_fuzzy_search(false);
        let queries = ["Network"];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(pins.is_some());
    }

    {
        let fields = vec![SearchType::TitleOnly];
        pinboard.enable_fuzzy_search(false);
        let queries = ["what", "*is*"];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(pins.is_some());
        dbg!(pins);
    }
}

#[test]
fn issue138_1_test() {
    let _ = env_logger::try_init();
    let mut myhome = rand_temp_path();
    myhome.push("mockito-rusty-pin");

    let cache_path = Some(myhome);
    debug!("create_mockito_servers: starting.");
    let _m1 = mock("GET", Matcher::Regex(r"^/posts/all.*$".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body_from_file("tests/issue-138-bookmark-1.json")
        .create();
    let _m2 = mock("GET", Matcher::Regex(r"^/tags/get.*$".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body_from_file("tests/issue-138-tags-1.json")
        .create();
    let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path)
        .expect("Can't setup Pinboard")
        .pinboard;
    let fields = vec![
        SearchType::TitleOnly,
        SearchType::TagOnly,
        SearchType::DescriptionOnly,
    ];
    {
        pinboard.enable_fuzzy_search(false);
        let queries = ["\u{c9c0}\u{ad6c}"]; // '지구'
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(pins.is_some());
        let queries = ["지구"];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(pins.is_some());
    }
    {
        pinboard.enable_fuzzy_search(false);
        let queries = ["\u{110c}\u{1175}\u{1100}\u{116e}"]; // '지구'
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(pins.is_some());
        let queries = ["지구"];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(pins.is_some());
    }
    {
        pinboard.enable_fuzzy_search(false);
        let queries = ["站,", "由", "个", "国", "Gmarket 韩国 No.1"];
        for query in queries {
            let pins = pinboard
                .search(&[query], &fields)
                .unwrap_or_else(|e| panic!("Finding {query} paniced: {e}"));
            assert!(pins.is_some(), "Couldn't find {query}");
            assert_eq!(1, pins.as_ref().unwrap().len(), "query: {query}");
        }
        let query = ["진흙"];
        let pins = pinboard
            .search(&query, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert_eq!(2, pins.as_ref().unwrap().len());
    }
}

#[test]
fn issue138_2_test() {
    let _ = env_logger::try_init();
    let mut myhome = rand_temp_path();
    myhome.push("mockito-rusty-pin");

    let cache_path = Some(myhome);
    debug!("create_mockito_servers: starting.");
    let _m1 = mock("GET", Matcher::Regex(r"^/posts/all.*$".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body_from_file("tests/issue-138-bookmark-2.json")
        .create();
    let _m2 = mock("GET", Matcher::Regex(r"^/tags/get.*$".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body_from_file("tests/issue-138-tags-2.json")
        .create();
    let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path)
        .expect("Can't setup Pinboard")
        .pinboard;
    let fields = vec![
        SearchType::TitleOnly,
        SearchType::TagOnly,
        SearchType::DescriptionOnly,
    ];
    {
        pinboard.enable_fuzzy_search(false);
        let queries = ["بید"];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(pins.is_none(), "Unexpectedly found: {queries:?}");
        let queries = [
            "آموزشی",
            "آموزش",
            "ب‌ید",
            "ب\u{200c}ید",
            "بخوانید",
            "بخوانی",
            "بخوا",
            "خوان",
        ];
        // Search one at a time
        for query in queries {
            let pins = pinboard
                .search(&[query], &fields)
                .unwrap_or_else(|e| panic!("Finding {query} paniced: {e}"));
            assert!(pins.is_some(), "Couldn't find {query}");
            assert_eq!(1, pins.as_ref().unwrap().len());
        }
        // Search all
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(pins.is_some());
    }
    {
        let fields = vec![
            SearchType::TitleOnly,
            SearchType::TagOnly,
            SearchType::DescriptionOnly,
            SearchType::UrlOnly,
        ];
        let queries = ["引き割り", "例", "納豆", "豆", "引き割り"];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(pins.is_some(), "Unable to find: {queries:?}");
    }
}

#[allow(clippy::too_many_lines)]
#[test]
fn test_issue7() {
    let _ = env_logger::try_init();
    let mut myhome = rand_temp_path();
    myhome.push("mockito-rusty-pin");

    let cache_path = Some(myhome);
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
    let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path)
        .expect("Can't setup Pinboard")
        .pinboard;
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
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(pins.is_some());
        assert_eq!(2, pins.unwrap().len());
        {
            let queries = ["iterm"];
            let fields = vec![SearchType::TitleOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_some());
            assert_eq!(1, pins.unwrap().len());
            let queries = ["iTerm"];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_some());
            assert_eq!(1, pins.unwrap().len());
        }
        {
            let queries = ["iterm"];
            let fields = vec![SearchType::TagOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_some());
            assert_eq!(2, pins.unwrap().len());
            let queries = ["iTerm"];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_some());
            assert_eq!(2, pins.unwrap().len());
        }
        {
            let queries = ["iterm"];
            let fields = vec![SearchType::DescriptionOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_none());
            let queries = ["iTerm"];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_none());
        }
        {
            let queries = ["iterm"];
            let fields = vec![SearchType::UrlOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_some());
            assert_eq!(1, pins.unwrap().len());
            let queries = ["iTerm"];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_some());
            assert_eq!(1, pins.unwrap().len());
        }
        {
            let queries = ["iterm2"];
            let fields = vec![SearchType::UrlOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_some());
            assert_eq!(1, pins.unwrap().len());
            let queries = ["iTerm2"];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_some());
            assert_eq!(1, pins.unwrap().len());
        }
        {
            let queries = ["iterm2"];
            let fields = vec![SearchType::TagOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_none());
            let queries = ["iTerm2"];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_none());
        }
        {
            let queries = ["iterm2"];
            let fields = vec![SearchType::TitleOnly];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!("{}", e));
            assert!(pins.is_some());
            assert_eq!(1, pins.unwrap().len());
            let queries = ["iTerm2"];
            let pins = pinboard
                .search(&queries, &fields)
                .unwrap_or_else(|e| panic!("{}", e));
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
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(pins.is_some());
        assert_eq!(1, pins.unwrap().len());
        let queries = ["Homebrew"];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(pins.is_some());
        assert_eq!(1, pins.unwrap().len());
        let queries = ["oh-my-zsh"];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(pins.is_some());
        assert_eq!(1, pins.unwrap().len());
    }
}

#[test]
fn issue138_3_test() {
    let _ = env_logger::try_init();
    let mut myhome = rand_temp_path();
    myhome.push("mockito-rusty-pin");

    let cache_path = Some(myhome);
    debug!("create_mockito_servers: starting.");
    let _m1 = mock("GET", Matcher::Regex(r"^/posts/all.*$".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body_from_file("tests/issue-138-bookmark-3.json")
        .create();
    let _m2 = mock("GET", Matcher::Regex(r"^/tags/get.*$".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body_from_file("tests/issue-138-tags-3.json")
        .create();
    let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path)
        .expect("Can't setup Pinboard")
        .pinboard;
    let fields = vec![SearchType::TagOnly];
    {
        pinboard.enable_fuzzy_search(false);
        let queries = [
            "آموزشی",
            "موزشی",
            "\u{0622}\u{0645}\u{0648}\u{0632}\u{0634}\u{06cc}",
        ];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        println!("{}", pins.as_ref().unwrap().len());
        assert!(pins.is_some());
    }
}

#[allow(clippy::too_many_lines)]
#[test]
fn search_multi_query_multi_field() {
    let _ = env_logger::try_init();
    debug!("search_multi_query_multi_field: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut myhome = rand_temp_path();
    myhome.push(".cache");
    myhome.push("mockito-rusty-pin");
    let cache_path = Some(myhome);

    let mut pinboard = Pinboard::new(include_str!("api_token.txt"), cache_path)
        .expect("Can't setup Pinboard")
        .pinboard;
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
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(pins.is_some());

        // Run same query, this time with Vec<String> instead of Vec<&str>
        let queries = vec!["eagle", "design", "fun"];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(pins.is_some());
    }

    // Find pins that have all keywords only in Title
    {
        let fields = vec![SearchType::TitleOnly];
        let queries = ["rust", "python"];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(pins.is_none());
    }

    // Find pins that have all keywords only in Url
    {
        let queries = ["pixlas"];
        let fields = vec![SearchType::UrlOnly];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
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
            .unwrap_or_else(|e| panic!("{}", e));
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
            .unwrap_or_else(|e| panic!("{}", e));
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
            .unwrap_or_else(|e| panic!("{}", e));
        assert_eq!(4, pins.as_ref().unwrap().len());
    }

    // Tag-only search
    {
        pinboard.enable_fuzzy_search(false);
        let queries = ["rust"];
        let fields = vec![SearchType::TagOnly];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert_eq!(10, pins.as_ref().unwrap().len());

        let queries = ["yubikey"];
        let fields = vec![SearchType::TagOnly];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert_eq!(3, pins.as_ref().unwrap().len());

        let queries = ["YubiKey"];
        let fields = vec![SearchType::TagOnly];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert_eq!(3, pins.as_ref().unwrap().len());

        let queries = ["keyboard", "hacks"];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
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
            .unwrap_or_else(|e| panic!("{}", e));
        assert_eq!(2, pins.as_ref().unwrap().len());
    }

    // title+url search non-fuzzy
    {
        pinboard.enable_fuzzy_search(false);
        let queries = ["世", "macos"];
        let fields = vec![SearchType::TitleOnly, SearchType::UrlOnly];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert_eq!(1, pins.as_ref().unwrap().len());
    }

    // url-only search non-fuzzy
    {
        pinboard.enable_fuzzy_search(false);
        let queries = ["ascii-table"];
        let fields = vec![SearchType::UrlOnly];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert_eq!(1, pins.as_ref().unwrap().len());
    }

    // url-only search fuzzy
    {
        pinboard.enable_fuzzy_search(true);
        let queries = ["ascii-table"];
        let fields = vec![SearchType::UrlOnly];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert_eq!(1, pins.as_ref().unwrap().len());
    }

    // empty search query
    {
        pinboard.enable_fuzzy_search(false);
        let queries = [""];
        let fields = vec![SearchType::TitleOnly, SearchType::UrlOnly];
        let pins = pinboard
            .search(&queries, &fields)
            .unwrap_or_else(|e| panic!("{}", e));
        assert!(pins.is_some());
    }
}

#[test]
fn serde_update_cache() {
    let _ = env_logger::try_init();
    debug!("serde_update_cache: starting.");
    let (_m1, _m2) = create_mockito_servers();
    let mut myhome = dirs::home_dir().unwrap();
    myhome.push(".cache");
    myhome.push("mockito-rusty-pin");
    let cache_path = Some(myhome.clone());

    // First remove all folders to force a full update
    fs::remove_dir_all(myhome).expect("Can't remove dir to prepare the test");

    let p = Pinboard::new(include_str!("api_token.txt"), cache_path);
    let mut pinboard = p.unwrap_or_else(|e| panic!("{e:?}")).pinboard;

    // Get all pins directly from Pinboard.in (no caching)
    let fresh_pins = pinboard.api.all_pins().expect("impossilbe?");

    pinboard.update_cache().expect("Couldn't update the cache");

    let cached_pins = pinboard.cached_data.pins.unwrap();
    assert_eq!(fresh_pins.len(), cached_pins.len());

    for (idx, fresh_pin) in fresh_pins.iter().enumerate() {
        info!("serde_update_cache: Checking pin[{}]", idx);
        let found = cached_pins
            .iter()
            .find(|&p| p.pin.url == fresh_pin.url);
        assert!(found.is_some(), "{fresh_pin:?}");
        let cached_pin = found.unwrap();
        // Title
        assert_eq!(fresh_pin.title, cached_pin.pin.title);
        assert_eq!(
            fresh_pin
                .title
                .nfkd()
                .collect::<String>()
                .to_lowercase(),
            cached_pin.title_lowered
        );
        // Url
        assert_eq!(fresh_pin.url, cached_pin.pin.url);
        // tags
        assert_eq!(fresh_pin.tags, cached_pin.pin.tags);
        assert_eq!(
            fresh_pin
                .tags
                .nfkd()
                .collect::<String>()
                .to_lowercase(),
            cached_pin.tag_list.join(" ")
        );
        // shared
        assert_eq!(fresh_pin.shared.to_lowercase(), cached_pin.pin.shared);
        // toread
        assert_eq!(fresh_pin.toread.to_lowercase(), cached_pin.pin.toread);
        // time
        assert_eq!(fresh_pin.time, cached_pin.pin.time);

        // extended
        if fresh_pin.extended.is_some() {
            assert!(cached_pin.pin.extended.is_some());
            assert_eq!(
                fresh_pin
                    .extended
                    .as_ref()
                    .unwrap()
                    .nfkd()
                    .collect::<String>(),
                cached_pin.pin.extended.as_ref().unwrap().as_ref()
            );
            assert_eq!(
                fresh_pin
                    .extended
                    .as_ref()
                    .unwrap()
                    .nfkd()
                    .collect::<String>()
                    .to_lowercase(),
                cached_pin.extended_lowered.as_ref().unwrap().as_ref()
            );
        } else {
            assert!(cached_pin.pin.extended.is_none());
        }
    }
}

// I am not sure why I wrote this test as it is kind of similar to serde_update_cache
#[test]
fn test_update_cache() {
    const IDX: usize = 25;

    let _ = env_logger::try_init();
    debug!("test_update_cache: starting.");

    let (_m1, _m2) = create_mockito_servers();
    let mut myhome = dirs::home_dir().unwrap();
    myhome.push(".cache");
    myhome.push("mockito-rusty-pin");

    let cache_path = Some(myhome.clone());

    debug!("Running first update_cache");

    // First remove all folders to force a full update
    fs::remove_dir_all(myhome).expect("Can't remove dir to prepare the test");

    // Pinboard::new() will call update_cache since we remove the cache folder.
    let pb = Pinboard::new(include_str!("api_token.txt"), cache_path);
    let pinboard = match pb {
        Ok(v) => v,
        Err(e) => panic!("{e:?}"),
    };
    let mut pinboard = pinboard.pinboard;
    let Some(pins) = pinboard.cached_data.pins.take() else { panic!("No pins found in cache!") };
    let Some(tags) = pinboard.cached_data.tags.take() else { panic!("No tags found in cache!") };
    assert!(pins.len() > IDX);
    assert!(tags.len() > IDX);

    debug!("Running second update_cache");
    pinboard
        .cached_data
        .update_cache(&pinboard.api)
        .unwrap_or_else(|e| panic!("{}", e));
    pinboard
        .cached_data
        .load_cache_data_from_file()
        .unwrap_or_else(|e| panic!("{}", e));
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
        let _ = pinboard
            .search_items(query)
            .unwrap_or_else(|e| panic!("{}", e));
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
            .unwrap_or_else(|e| panic!("{}", e));
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
            .unwrap_or_else(|e| panic!("{}", e));
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
            .unwrap_or_else(|e| panic!("{}", e));
    });
}
