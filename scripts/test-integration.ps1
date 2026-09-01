Set-Item -Path Env:DATABASE_URL -Value "postgres://postgres:postgres@localhost:5434/feen_test"
cargo test integration_tests -- --test-threads=1 @args
