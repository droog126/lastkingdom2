set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default:
    @just --list

build:
    cargo run -q -p xtask -- dev build

build-client:
    cargo build -p lk2-client

kill:
    Get-Process lk2-client,lk2-server -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

kill-build:
    Get-Process lk2-client,lk2-server,cargo,rustc,rustdoc -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

server:
    cargo run -p lk2-server

client:
    $env:BEVY_DISABLE_ACCESSIBILITY="1"; cargo run -p lk2-client -- --offline

model-preview:
    cargo run -p lk2-client -- --model-preview

# Iterate every GLB under assets/, screenshot each individually, and write a
# self-evaluation summary to screenshots/model_preview/decision.md. Use this
# when you want to spot-check the whole catalog without manually clicking
# through every featured model.
model-preview-all:
    cargo run -q -p xtask -- model-preview-all

# Same as model-preview-all but only renders the named model (stem or path).
model-preview-all-only MODEL:
    cargo run -q -p xtask -- model-preview-all --only={{MODEL}}

model-preview-one MODEL:
    cargo run -p lk2-client -- --model-preview --model-preview-one={{MODEL}}

model-preview-shot MODEL:
    cargo run -p lk2-client -- --model-preview --model-preview-one={{MODEL}} --model-preview-shot

terrain-preview:
    cargo run -p lk2-client -- --terrain-preview

terrain-preview-shot:
    cargo run -p lk2-client -- --terrain-preview --terrain-preview-shot

client-online:
    $env:BEVY_DISABLE_ACCESSIBILITY="1"; Get-Process lk2-server -ErrorAction SilentlyContinue | Stop-Process -Force; cargo build -p lk2-server -p lk2-client; Start-Process -FilePath .\target\debug\lk2-server.exe -WorkingDirectory (Get-Location) -WindowStyle Hidden; Start-Sleep -Seconds 2; .\target\debug\lk2-client.exe --connect=127.0.0.1:5000 --first-person

offline:
    $env:BEVY_DISABLE_ACCESSIBILITY="1"; cargo run -p lk2-client -- --offline

offline-fast:
    cargo run -q -p xtask -- dev stage-runtime
    .\target\debug\lk2-client.exe --offline

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
