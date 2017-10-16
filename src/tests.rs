use super::*;

#[test]
fn set_tags() {
    let url = Url::parse("https://githubуй.com/Здравствуйтеу").unwrap();
    let mut p = Pin::new(url, "title".to_string(), vec![], true, false, None);

    let tags = vec!["tag1", "tag2"];
    p.set_tags_str(&tags);
    assert_eq!(p.tags, tags);
    p.title = "fuck".to_string();

    let tags = vec![String::from("tag3"), "tag4".to_string()];
    p.set_tags_str(
        tags.iter()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .as_slice(),
    );
    assert_eq!(p.tags, tags);

    let tags = vec![String::from("tag5"), "tag6".to_string()];
    p.set_tags(tags.clone());
    assert_eq!(p.tags, tags);
}
