# This script takes care of building & testing your crate

set -ex

# TODO This is the "test phase", tweak it as you see fit
tests() {
    mkdir -p ~/.cache/mockito-rusty-pin
    export RUST_LOG=rusty_pin=debug
    case "$TARGET" in
        x86_64-unknown-linux-gnu)
            cargo test --target "$TARGET" -- --nocapture --test-threads=1
            ;;
        *)
            cargo test --target "$TARGET" -- --nocapture --test-threads=1
    esac
}

main() {
    if [ -n "$DISABLE_TESTS" ] || [ -z "$CIRCLECI_TEST" ]; then
        cargo build --target "$TARGET"
        return
    else
        tests
    fi
}


main

