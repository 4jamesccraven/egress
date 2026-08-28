daemon:
    cargo run --bin egressd

client *args:
    cargo run --bin egressctl -- {{ args }}
