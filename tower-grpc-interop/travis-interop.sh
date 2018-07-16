#!/usr/bin/env bash
# script to run interop tests on CI.
set -eu
set -o pipefail

SERVER="interop-server-go-linux-amd64"
SERVER_ZIP_URL="https://github.com/tower-rs/tower-grpc/files/1616271/interop-server-go-linux-amd64.zip"

# download test server from grpc-go
if ! [ -e "${SERVER}" ] ; then
    echo ":; downloading grpc-go test server"
    wget -O "${SERVER}.zip" "${SERVER_ZIP_URL}"
    unzip "${SERVER}.zip"
fi

# run the test server
./"${SERVER}" &
SERVER_PID=$!
echo ":; started grpc-go test server."

# trap exits to make sure we kill the server process when the script exits,
# regardless of why (errors, SIGTERM, etc).
trap 'echo ":; killing test server"; kill ${SERVER_PID};' EXIT

# run the interop test client against the server.
cargo run -p tower-grpc-interop --bin client -- \
    --test_case=empty_unary,large_unary,client_streaming
