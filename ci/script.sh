# This script takes care of testing your crate

set -ex

# TODO This is the "test phase", tweak it as you see fit
main() {

    if [ ! -z "$DISABLE_TESTS" ]; then
        cross build --target "$TARGET"
        return
    fi

    # cross test --target $TARGET
    mkdir -p ~/.cache/mockito-rusty-pin
    export RUST_LOG=rusty_pin=debug
    case "$TARGET" in
        x86_64-unknown-linux-gnu)
            cargo test --target "$TARGET" -- --nocapture --test-threads=1
            ;;
        *)
            cross test --target "$TARGET" -- --nocapture --test-threads=1
    esac
}

main

