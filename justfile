set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default:
    @just --list

xtask *ARGS:
    $env:CARGO_INCREMENTAL="0"; cargo run -p xtask --target-dir .tmp/xtask-target -- {{ARGS}}

build:
    just xtask dev build

build-client:
    cargo build -p lk2-client

kill:
    $ps = Get-Process lk2-client,lk2-server -ErrorAction SilentlyContinue; if ($ps) { $ps | Stop-Process -Force -ErrorAction SilentlyContinue }; exit 0

kill-build:
    $ps = Get-Process lk2-client,lk2-server,cargo,rustc,rustdoc -ErrorAction SilentlyContinue; if ($ps) { $ps | Stop-Process -Force -ErrorAction SilentlyContinue }; exit 0

server:
    cargo run -p lk2-server

server-fast:
    $env:CARGO_BUILD_JOBS="12"; cargo run -p lk2-server

client *ARGS:
    $env:CARGO_BUILD_JOBS="4"; just play {{ARGS}}

client-fast *ARGS:
    $env:CARGO_BUILD_JOBS="12"; just play {{ARGS}}

clint *ARGS:
    just client {{ARGS}}

play *ARGS:
    just xtask play {{ARGS}}

model-preview:
    just play --gpu-backend=vulkan --model-preview

# Iterate every GLB under assets/, screenshot each individually, and write a
# self-evaluation summary to screenshots/model_preview/decision.md. Use this
# when you want to spot-check the whole catalog without manually clicking
# through every featured model.
model-preview-all:
    just xtask model-preview-all

# Same as model-preview-all but only renders the named model (stem or path).
model-preview-all-only MODEL:
    just xtask model-preview-all --only={{MODEL}}

model-preview-one MODEL:
    just play --gpu-backend=vulkan --model-preview --model-preview-one={{MODEL}}

model-preview-shot MODEL:
    just play --gpu-backend=vulkan --model-preview --model-preview-one={{MODEL}} --model-preview-shot

game-scene-shot:
    just play --gpu-backend=vulkan --game-scene-shot

terrain-preview:
    just play --terrain-preview

terrain-preview-shot:
    just play --terrain-preview --terrain-preview-shot

offline:
    just play

offline-fast:
    just play --skip-build

build-server:
    cargo build -p lk2-server

build-full:
    cargo build -p lk2-client
    cargo build -p lk2-server

release-client:
    cargo build --release -p lk2-client

release-server:
    cargo build --release -p lk2-server

release-full:
    cargo build --release -p lk2-client
    cargo build --release -p lk2-server

test:
    just xtask tdd --scope workspace

test-core:
    just xtask tdd --scope core

test-client:
    just xtask tdd --scope client

test-server:
    just xtask tdd --scope server

test-changed:
    just xtask tdd --scope changed

test-nextest:
    cargo nextest run --workspace

coverage:
    cargo llvm-cov nextest --workspace --summary-only

deps-unused:
    cargo machete

snapshots:
    cargo insta test -p lk2-core

snapshots-review:
    cargo insta review

fmt:
    just xtask tdd --scope fmt

fmt-fix:
    cargo fmt --all

clippy:
    just xtask tdd --scope clippy

clean:
    cargo clean

clean-runs:
    just xtask clean-runs

loop:
    just xtask loop --offline --seconds 60

loop-online:
    just xtask loop --online --refresh-after-fail --first-person --seconds 60

loop-skip-build:
    just xtask loop --offline --skip-build --seconds 60

health:
    just xtask health

health-iter ITER_DIR:
    just xtask health {{ITER_DIR}}

scenario JSON:
    just xtask scenario --json {{JSON}}

export-content *ARGS:
    just xtask export-content {{ARGS}}

export *ARGS:
    just xtask export-content {{ARGS}}

audit-tdd:
    just xtask audit-tdd

audit-architecture:
    just xtask audit-architecture

_xtask-audit *ARGS:
    $env:CARGO_INCREMENTAL="0"; cargo run -q -p xtask --target-dir .tmp/xtask-audit-target -- {{ARGS}}

audit-docs:
    just _xtask-audit audit-docs

audit-skills:
    just _xtask-audit audit-skills

help:
    just xtask help
