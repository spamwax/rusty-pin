# This script takes care of testing your crate

set -ex

# TODO This is the "test phase", tweak it as you see fit
main() {
    cross build --target "$TARGET"

    if [ ! -z "$DISABLE_TESTS" ]; then
        return
    fi

    # cross test --target $TARGET
    mkdir -p ~/.cache/mockito-rusty-pin
    export RUST_LOG=rusty_pin=debug
    cross test --target "$TARGET" -- --nocapture --test-threads=1
    # cargo test --target "$TARGET" -- --nocapture --test-threads=1

}

main

