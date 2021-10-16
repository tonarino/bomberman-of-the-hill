#!/bin/bash

# TODO(Matej): load from .env instead
UPLOAD_SERVER_ADDRESS=localhost:8765
API_KEY=abcdef
CRATE_NAME=wanderer

set -o errexit
set -o pipefail

# TODO(Matej): use release build instead? (does it make difference for WASM?)
cargo build -p ${CRATE_NAME} --target wasm32-unknown-unknown

curl -X POST -H "Api-Key: ${API_KEY}" --data-binary \
    @target/wasm32-unknown-unknown/debug/${CRATE_NAME}.wasm http://${UPLOAD_SERVER_ADDRESS}/
