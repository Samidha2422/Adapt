set shell := ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"]

default: check

bootstrap:
    @& ./scripts/just.ps1 bootstrap
fmt:
    @& ./scripts/just.ps1 fmt
format:
    @& ./scripts/just.ps1 format
lint:
    @& ./scripts/just.ps1 lint
test:
    @& ./scripts/just.ps1 test
check:
    @& ./scripts/just.ps1 check
build:
    @& ./scripts/just.ps1 build
lock:
    @& ./scripts/just.ps1 lock
run:
    @& ./scripts/just.ps1 run
mock:
    @& ./scripts/just.ps1 mock
openapi:
    @& ./scripts/just.ps1 openapi
seed:
    @& ./scripts/just.ps1 seed
demo-seed:
    @& ./scripts/just.ps1 demo-seed
eval:
    @& ./scripts/just.ps1 eval
bench:
    @& ./scripts/just.ps1 bench
models-verify:
    @& ./scripts/just.ps1 models-verify
models-verify-test:
    @& ./scripts/just.ps1 models-verify-test
models-verify-install:
    @& ./scripts/just.ps1 models-verify-install
train-download:
    @& ./scripts/just.ps1 train-download
train-export:
    @& ./scripts/just.ps1 train-export
train-behavior:
    @& ./scripts/just.ps1 train-behavior
