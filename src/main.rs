#![feature(core_intrinsics)]

extern crate rusty_pin;

fn main() {
    println!("Hello, world!");
}

#[allow(dead_code)]
fn print_type_of<T>(_: &T) {
    println!("{}", unsafe { std::intrinsics::type_name::<T>() });
}
