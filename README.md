Run proxy server that listens to http requests, 
records requests and responses in 
[PACT](https://docs.pact.io/) format and sends
back the response to the sender.

Useful in unit tests.
First time test
fetches the data from the web and runs slow, but after that
it runs fast because it uses the recorded data.

The request sent to it has to be a GET request
in the following format:

    http://localhost:<port>/<scheme>/<host>/<path>?<query>

e.g:
    
    http://localhost:8080/http/data.binance.com/api/v3/klines?symbol=ZECUSDT&interval=1w&limit=1

### Usage

1) For Rust app. As a rust wrapper for pact mock server

Add to cargo build
```
[dependencies]
pact-proxy-rs = { git = "https://github.com/NikZak/pact-proxy-rs.git"}
```
and the in your test
```
    use pact_proxy_rs::server::PactServer;
    
    let mut pact_server = PactServer::with_http_server(&PathBuf::from("tests/pacts"), None, None).unwrap();
    let port = pact_server.port().unwrap();
    pact_server.start_non_blocking().await.unwrap();
    ...
    let client = MyClient::new_with_url("http://localhost:".to_owned() + port.as_str() + "/https/yoururl.com"));
    let data = client.get_data();
    ...
    pact_server.stop();
```
if you don't issue `pact_serfer.stop()` instruction in the end then test won't finish 
as it would still have a server waiting for requests.

2) For non-Rust app. As a standalone server.

```commandline
cargo run -- [OPTIONS] --pact_files_folder <FILE> -p <PORT>
```
or
```commandline
cargo build
target/debug/pact-proxy-rs [OPTIONS] --pact_files_folder <FILE> -p <PORT>
```

Options:
```
  -f, --pact_files_folder <FILE>  The folder where pacts files will be recorded
  -p, --port <PORT>               (Optional) The port to run the mock service on, if not set then random port is chose
```
and the in your test
```
    ...
    let client = MyClient::new_with_url("http://localhost:".to_owned() + <PORT>.as_str() + "/https/yoururl.com"));
    let data = client.get_data();
    ...
```

In this mode this is run as a standalone server on localhost on a given port.
The server captures requests sent to it and transfers them to target-url.
Every received response is recorded to pact file together with request.

### Contribution

Looking for contributors. Contributions are welcome.

Done:
- [x] Transform Http GET request and response to PACT format
- [x] Record PACT files
- [x] Serve PACT files
- [x] Match PACT interaction
- [x] Transform from PACT format to Http response
- [x] Run as a standalone server

Not done:
- [ ] Transform Http POST request and response to PACT format (requires changing interaction key/descr to a hash)
- [ ] Accept websocket requests
- [ ] Documentation
- [ ] More tests coverage

Unless you explicitly state otherwise, any contribution 
intentionally submitted for inclusion in this repository
by you, as defined in the Apache-2.0 license, shall be 
licensed as Apache-2.0 as above, without any additional 
terms or conditions.

