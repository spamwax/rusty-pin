// #![allow(clippy::must_use_candidate)]
use std::cmp::Ordering;
use std::fmt;

extern crate serde;

// use rmps::{Deserializer, Serializer};
// use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Eq, Clone)]
pub struct Tag(pub String, pub TagFreq);

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum TagFreq {
    Used(usize),
    New,
    Popular,
}

impl Tag {
    #[must_use]
    pub fn new(tag: String, freq: usize) -> Self {
        Tag(tag, TagFreq::Used(freq))
    }

    #[must_use]
    pub fn set_popular(mut self) -> Self {
        self.1 = TagFreq::Popular;
        self
    }

    #[must_use]
    pub fn set_new(mut self) -> Self {
        self.1 = TagFreq::New;
        self
    }
}

impl PartialOrd for Tag {
    fn partial_cmp(&self, other: &Tag) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Tag {
    fn cmp(&self, other: &Tag) -> Ordering {
        if self.1 == other.1 {
            self.0.cmp(&other.0).reverse()
        } else {
            self.1.cmp(&other.1)
        }
    }
}

impl PartialEq for Tag {
    fn eq(&self, other: &Tag) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl fmt::Display for TagFreq {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TagFreq::New => write!(f, "NEW TAG"),
            TagFreq::Popular => write!(f, "Popular"),
            TagFreq::Used(n) => write!(f, "{}", n),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_displays_tagfreq() {
        let t = TagFreq::Used(12);
        assert_eq!("12", t.to_string().as_str());

        let t = TagFreq::Used(0);
        assert_eq!("0", t.to_string().as_str());

        let t = TagFreq::New;
        assert_eq!("NEW TAG", t.to_string().as_str());

        let t = TagFreq::Popular;
        assert_eq!("Popular", t.to_string().as_str());
    }

    #[test]
    fn tag_unicode_test() {
        let t1 = Tag::new("tag👻1".to_string(), 1);
        let t2 = Tag::new("tag👻2".to_string(), 1);
        assert!(t1 != t2);
        let t3 = Tag::new("TaG👻2".to_string(), 1);
        assert!(t2 == t3);
    }

    #[test]
    fn it_sorts_tagfreq() {
        #[allow(clippy::nonminimal_bool)]
        fn verify_partialord(t1: &TagFreq, t2: &TagFreq) {
            assert!(t1 < t2);
            assert!(!(t1 > t2));
            assert!(t1 != t2);
        }

        let t1 = TagFreq::Used(1);
        let t2 = TagFreq::Used(2);
        verify_partialord(&t1, &t2);

        let t1 = TagFreq::Used(2);
        let t2 = TagFreq::Used(1);
        verify_partialord(&t2, &t1);

        let t1 = TagFreq::Used(1);
        let t2 = TagFreq::Popular;
        verify_partialord(&t1, &t2);

        let t1 = TagFreq::Used(1);
        let t2 = TagFreq::New;
        verify_partialord(&t1, &t2);

        let t1 = TagFreq::Popular;
        let t2 = TagFreq::New;
        verify_partialord(&t2, &t1);
    }
}
