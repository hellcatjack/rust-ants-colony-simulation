$env:CARGO_TARGET_DIR = "$env:TEMP\rust-ants-colony-target"
Write-Host "Building in temporary directory: $env:CARGO_TARGET_DIR"
cargo run --release
