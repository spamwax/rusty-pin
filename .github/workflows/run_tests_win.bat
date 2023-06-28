@echo off

set RUST_BACKTRACE=1
set RUST_LOG=rusty_pin=debug

set working_dir="%GITHUB_WORKSPACE%"

cargo clippy --tests --workspace -- -Dclippy::all -Dclippy::pedantic -D warnings
powershell -nop -c "& {sleep 4}"
echo "=============================================================================================="
echo "=============================================================================================="
echo " "
cargo test -- --test-threads=1 --nocapture
