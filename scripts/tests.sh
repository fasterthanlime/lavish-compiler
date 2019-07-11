#!/bin/bash -xe

cargo run --quiet --manifest-path tests/lavish-test-runner/Cargo.toml -- "$@"

