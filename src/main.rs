#![feature(core_intrinsics)]
extern crate url;

use url::Url;

#[derive(Debug)]
struct Pin {
    url: Url,
    title: String,
    tags: Vec<String>,
    private: bool,
    read: bool,
    desc: Option<String>,
}

impl Pin {
    fn new(url: Url, title: String, tags: Vec<String>,
           private: bool,
           read: bool,
           desc: Option<String>) -> Pin {
        Pin { url, title, tags, private, read, desc }
    }
}

fn main() {
    println!("Hello, world!");
}

fn print_type_of<T>(_: &T) {
    println!("{}", unsafe { std::intrinsics::type_name::<T>() });
}
