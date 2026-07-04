set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default:
    @just --list

build:
    cargo run -q -p xtask -- dev build

build-client:
    cargo build -p lk2-client --features dev-dynamic-linking,lk2-core/dev-dynamic-linking

server:
    cargo run -p lk2-server --features dev-dynamic-linking

client:
    cargo run -p lk2-client --features dev-dynamic-linking,lk2-core/dev-dynamic-linking -- --connect=127.0.0.1:5000

offline:
    cargo run -p lk2-client -- --offline

build-server:
    cargo build -p lk2-server --features dev-dynamic-linking

build-full:
    cargo build -p lk2-client --features dev-dynamic-linking,lk2-core/dev-dynamic-linking
    cargo build -p lk2-server --features dev-dynamic-linking

release-client:
    cargo build --release -p lk2-client

release-server:
    cargo build --release -p lk2-server

release-full:
    cargo build --release -p lk2-client
    cargo build --release -p lk2-server

test:
    cargo run -q -p xtask -- tdd --scope workspace

test-core:
    cargo run -q -p xtask -- tdd --scope core

test-client:
    cargo run -q -p xtask -- tdd --scope client

test-server:
    cargo run -q -p xtask -- tdd --scope server

test-changed:
    cargo run -q -p xtask -- tdd --scope changed

fmt:
    cargo run -q -p xtask -- tdd --scope fmt

fmt-fix:
    cargo fmt --all

clippy:
    cargo run -q -p xtask -- tdd --scope clippy

clean:
    cargo clean

loop:
    cargo run -q -p xtask -- loop --offline --seconds 60

loop-online:
    cargo run -q -p xtask -- loop --online --seconds 60

loop-skip-build:
    cargo run -q -p xtask -- loop --offline --skip-build --seconds 60

health:
    cargo run -q -p xtask -- health

health-iter ITER_DIR:
    cargo run -q -p xtask -- health {{ITER_DIR}}

scenario JSON:
    cargo run -q -p xtask -- scenario --json {{JSON}}

audit-tdd:
    cargo run -q -p xtask -- audit-tdd

audit-architecture:
    cargo run -q -p xtask -- audit-architecture

help:
    cargo run -q -p xtask -- help
