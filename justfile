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
integration-smoke:
    @& ./scripts/just.ps1 integration-smoke
api-test:
    @& ./scripts/just.ps1 api-test
api-lint:
    @& ./scripts/just.ps1 api-lint
audio-worker-test:
    @& ./scripts/just.ps1 audio-worker-test
models-admit-known:
    @& ./scripts/just.ps1 models-admit-known
inspect-whisper:
    @& ./scripts/just.ps1 inspect-whisper
whisper-smoke:
    @& ./scripts/just.ps1 whisper-smoke
whisper-deps-install:
    @& ./scripts/just.ps1 whisper-deps-install
python-version:
    @& ./scripts/just.ps1 python-version
multimodal-smoke:
    @& ./scripts/just.ps1 multimodal-smoke
whisper-api-smoke:
    @& ./scripts/just.ps1 whisper-api-smoke
training-test:
    @& ./scripts/just.ps1 training-test
models-verify-install:
    @& ./scripts/just.ps1 models-verify-install
train-download:
    @& ./scripts/just.ps1 train-download
train-export:
    @& ./scripts/just.ps1 train-export
train-behavior:
    @& ./scripts/just.ps1 train-behavior
