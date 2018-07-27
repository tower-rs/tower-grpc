# tower-grpc-interop

The [gRPC interoperability test cases](https://github.com/grpc/grpc/blob/master/doc/interop-test-descriptions.md) for `tower-grpc`.

## Checklist

Note that currently, only the interop test client is implemented. The `docker-compose.yml` in this directory will run the `tower-grpc` interop client against the test server from `grpc-go`.

- [x] `empty_unary`: implemented in client
- [ ] `cacheable_unary`: started, requires request context implementation to set cacheable flag
- [x] `large_unary`: implemented in client
- [ ] ~`client_compressed_unary`~: requires gRPC compression, NYI
- [ ] ~`server_compressed_unary`~: requires gRPC compression, NYI
- [x] `client_streaming`: implemented in client
- [ ] ~`client_compressed_streaming`~: requires gRPC compression, NYI
- [x] `server_streaming`
- [ ] ~`server_compressed_streaming`~: requires gRPC compression, NYI
- [x] `ping_pong`
- [x] `empty_stream`
- [ ] ~`compute_engine_creds`~ requires auth, NYI
- [ ] ~`jwt_token_creds`~ requires auth, NYI
- [ ] ~`oauth2_auth_token`~ requires auth, NYI
- [ ] ~`per_rpc_creds`~ requires auth, NYI
- [ ] `custom_metadata`
- [ ] `status_code_and_message`
- [ ] `special_status_message`
- [x] `unimplemented_method`
- [x] `unimplemented_service`
- [ ] `cancel_after_begin`
- [ ] `cancel_after_first_response`
- [ ] `timeout_on_sleeping_server`
- [ ] `concurrent_large_unary`

## Running

Run the test client:

```bash
$ cargo run -p tower-grpc-interop --bin client -- --help
interop-client
Eliza Weisman <eliza@buoyant.io>

USAGE:
    client [FLAGS] [OPTIONS]

FLAGS:
    -h, --help           Prints help information
        --use_test_ca    Whether to replace platform root CAs with ca.pem as the CA root.
    -V, --version        Prints version information

OPTIONS:
        --ca_file <FILE>                             The file containing the CA root cert file [default: ca.pem]
        --default_service_account <ACCOUNT_EMAIL>    Email of the GCE default service account.
        --oauth_scope <SCOPE>
            The scope for OAuth2 tokens. For example, "https://www.googleapis.com/auth/xapi.zoo".

        --server_host <HOSTNAME>
            The server host to connect to. For example, "localhost" or "127.0.0.1" [default: 127.0.0.1]

        --server_host_override <HOSTNAME>
            The server host to claim to be connecting to, for use in TLS and HTTP/2 :authority header. If unspecified,
            the value of `--server_host` will be used
        --server_port <PORT>
            The server port to connect to. For example, "8080". [default: 10000]

        --service_account_key_file <PATH>
            The path to the service account JSON key file generated from GCE developer console.

        --test_case <TESTCASE>
            The name of the test case to execute. For example,
                            "empty_unary". [default: large_unary]  [values: empty_unary, cacheable_unary, large_unary,
            client_compressed_unary, server_compressed_unary, client_streaming, client_compressed_streaming,
            server_streaming, server_compressed_streaming, ping_pong, empty_stream, compute_engine_creds,
            jwt_token_creds, oauth2_auth_token, per_rpc_creds, custom_metadata, status_code_and_message,
            special_status_message, unimplemented_method, unimplemented_service, cancel_after_begin,
            cancel_after_first_response, timeout_on_sleeping_server, concurrent_large_unary]
        --use_tls <BOOLEAN>
            Whether to use a plaintext or encrypted connection. [default: false]  [values: true, false]
```

Run the test server (currently not yet implemented):

```bash
$ cargo run -p tower-grpc-interop --bin server
```

The `docker-compose.yml` in this directory can also be used to run the `tower-grpc` test client against `grpc-go`'s test server. From the repository root directory:

```bash
$ docker-compose --file=tower-grpc-interop/docker-compose.yml up --exit-code-from client-tower
```

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in `tower-grpc` by you, shall be licensed as MIT, without any
additional terms or conditions.
