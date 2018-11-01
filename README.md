# Docker Container builder for Rust ecosystem

## Hacking and Debugging
So far the debugging and testing workflow is next:

0. Ensure Docker uses BuildKit (this provides an incredible build time improvement).

1. Create and inspect build plan:
```
cargo build -Z unstable-options --build-plan --manifest-path=examples/workspace/Cargo.toml --all-targets > plan.json
```

2. Generate and inspect Dockerfile:
```
cargo run --bin cargo-wharf -- --crate-root examples/workspace generate plan.json --template examples/workspace/Dockerfile.hbs > examples/workspace/Dockerfile
```

3. Build `container-tools` image:
```
docker build -t denzp/cargo-container-tools:`cargo pkgid --manifest-path=cargo-container-tools/Cargo.toml | cut -d\# -f2 | cut -d: -f2` .
```

4. Build the example image:
```
docker build -t TAG examples/workspace
```

## Final Usage
Sure, in real life workflow would be different (assuming `build-plan` cargo feature became stable):
```
cargo build --build-plan --all-targets | cargo wharf generate | docker build -t TAG -f - .
```

Or high-level API:
```
cargo wharf build -t TAG1 -t TAG2
cargo wharf test
```