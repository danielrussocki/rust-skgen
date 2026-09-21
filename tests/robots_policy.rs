use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use rust_skgen::{
    fetch::{FetchConfiguration, HttpDocumentFetcher},
    policy::{CrawlPolicy, RobotsTxtPolicy},
};
use url::Url;

struct TestServer {
    address: String,
    worker: thread::JoinHandle<()>,
}

impl TestServer {
    fn start(status: &'static str, robots_txt: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .unwrap_or_else(|error| panic!("failed to bind test server: {error}"));
        let address = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("failed to read test server address: {error}"))
            .to_string();
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener
                .accept()
                .unwrap_or_else(|error| panic!("failed to accept robots request: {error}"));
            let request = read_request(&mut stream);
            assert!(request.starts_with("GET /robots.txt HTTP/1.1\r\n"));
            respond(&mut stream, status, robots_txt);
        });

        Self { address, worker }
    }

    fn url(&self, path: &str) -> Url {
        Url::parse(&format!("http://{}{path}", self.address))
            .unwrap_or_else(|error| panic!("invalid test URL: {error}"))
    }

    fn start_inaccessible() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .unwrap_or_else(|error| panic!("failed to bind test server: {error}"));
        let address = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("failed to read test server address: {error}"))
            .to_string();
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener
                .accept()
                .unwrap_or_else(|error| panic!("failed to accept robots request: {error}"));
            let _ = read_request(&mut stream);
        });

        Self { address, worker }
    }

    fn join(self) {
        self.worker
            .join()
            .unwrap_or_else(|_| panic!("test server worker panicked"));
    }
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0; 1024];

    loop {
        let bytes_read = stream
            .read(&mut buffer)
            .unwrap_or_else(|error| panic!("failed to read request: {error}"));
        request.extend_from_slice(&buffer[..bytes_read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8(request)
                .unwrap_or_else(|error| panic!("request was not UTF-8: {error}"));
        }
    }
}

fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap_or_else(|error| panic!("failed to send response: {error}"));
    stream
        .flush()
        .unwrap_or_else(|error| panic!("failed to flush response: {error}"));
}

fn policy(requires_robots_txt: bool) -> RobotsTxtPolicy<HttpDocumentFetcher> {
    let user_agent = "rust-skgen-test";
    let fetcher = HttpDocumentFetcher::new(FetchConfiguration::new(
        user_agent,
        Duration::from_secs(1),
        0,
    ))
    .unwrap_or_else(|error| panic!("failed to create fetcher: {error}"));
    RobotsTxtPolicy::new(fetcher, user_agent, requires_robots_txt)
}

#[test]
fn permits_a_url_allowed_for_the_configured_user_agent() {
    let server = TestServer::start(
        "200 OK",
        "User-agent: rust-skgen-test\nAllow: /docs/\nDisallow: /\n",
    );

    let allowed = policy(false)
        .allows(&server.url("/docs/overview"))
        .unwrap_or_else(|error| panic!("failed to evaluate robots policy: {error}"));

    assert!(allowed);
    server.join();
}

#[test]
fn forbids_a_url_disallowed_for_the_configured_user_agent() {
    let server = TestServer::start(
        "200 OK",
        "User-agent: rust-skgen-test\nDisallow: /private/\n",
    );

    let allowed = policy(false)
        .allows(&server.url("/private/draft"))
        .unwrap_or_else(|error| panic!("failed to evaluate robots policy: {error}"));

    assert!(!allowed);
    server.join();
}

#[test]
fn unavailable_robots_txt_is_optional_unless_required() {
    let optional_server = TestServer::start("404 Not Found", "missing");
    let optional = policy(false)
        .allows(&optional_server.url("/docs/overview"))
        .unwrap_or_else(|error| panic!("optional robots policy should allow discovery: {error}"));
    optional_server.join();

    let required_server = TestServer::start("404 Not Found", "missing");
    let required = policy(true).allows(&required_server.url("/docs/overview"));
    required_server.join();

    assert!(optional);
    assert!(required.is_err());
}

#[test]
fn inaccessible_robots_txt_is_optional_unless_required() {
    let optional_server = TestServer::start_inaccessible();
    let optional = policy(false)
        .allows(&optional_server.url("/docs/overview"))
        .unwrap_or_else(|error| panic!("optional robots policy should allow discovery: {error}"));
    optional_server.join();

    let required_server = TestServer::start_inaccessible();
    let required = policy(true).allows(&required_server.url("/docs/overview"));
    required_server.join();

    assert!(optional);
    assert!(required.is_err());
}

#[test]
fn malformed_robots_txt_is_optional_unless_required() {
    let optional_server = TestServer::start("200 OK", "not valid robots data");
    let optional = policy(false)
        .allows(&optional_server.url("/docs/overview"))
        .unwrap_or_else(|error| panic!("optional robots policy should allow discovery: {error}"));
    optional_server.join();

    let required_server = TestServer::start("200 OK", "not valid robots data");
    let required = policy(true).allows(&required_server.url("/docs/overview"));
    required_server.join();

    assert!(optional);
    assert!(required.is_err());
}
