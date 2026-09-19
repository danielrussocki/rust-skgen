use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Barrier, Mutex},
    thread,
    time::Duration,
};

use rust_skgen::fetch::{DocumentFetcher, FetchConfiguration, HttpDocumentFetcher};
use url::Url;

struct TestServer {
    address: String,
    worker: thread::JoinHandle<()>,
}

impl TestServer {
    fn start(requests: usize, handler: impl Fn(TcpStream) + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let handler = Arc::new(handler);
        let worker = thread::spawn(move || {
            let mut workers = Vec::with_capacity(requests);

            for stream in listener.incoming().take(requests) {
                let handler = Arc::clone(&handler);
                workers.push(thread::spawn(move || handler(stream.unwrap())));
            }

            for worker in workers {
                worker.join().unwrap();
            }
        });

        Self { address, worker }
    }

    fn url(&self, path: &str) -> Url {
        Url::parse(&format!("http://{}{path}", self.address)).unwrap()
    }

    fn join(self) {
        self.worker.join().unwrap();
    }
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0; 1024];

    loop {
        let bytes_read = stream.read(&mut buffer).unwrap();
        request.extend_from_slice(&buffer[..bytes_read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8(request).unwrap();
        }
    }
}

fn respond(stream: &mut TcpStream, status: &str, headers: &[(&str, &str)], body: &str) {
    if write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\n",
        body.len()
    )
    .is_err()
    {
        return;
    }
    for (name, value) in headers {
        if write!(stream, "{name}: {value}\r\n").is_err() {
            return;
        }
    }
    if write!(stream, "Connection: close\r\n\r\n{body}").is_ok() {
        let _ = stream.flush();
    }
}

#[test]
fn sends_the_configured_user_agent() {
    let received_request = Arc::new(Mutex::new(String::new()));
    let server_request = Arc::clone(&received_request);
    let server = TestServer::start(1, move |mut stream| {
        *server_request.lock().unwrap() = read_request(&mut stream);
        respond(&mut stream, "200 OK", &[], "documentation");
    });
    let fetcher = HttpDocumentFetcher::new(FetchConfiguration::new(
        "rust-skgen-test/1.0",
        Duration::from_secs(1),
        0,
    ))
    .unwrap();

    let response = fetcher.fetch(server.url("/guide")).unwrap();

    assert_eq!(response.status(), 200);
    server.join();
    assert!(
        received_request
            .lock()
            .unwrap()
            .to_ascii_lowercase()
            .contains("user-agent: rust-skgen-test/1.0\r\n")
    );
}

#[test]
fn returns_redirect_responses_without_following_them() {
    let server = TestServer::start(1, move |mut stream| {
        read_request(&mut stream);
        respond(&mut stream, "302 Found", &[("Location", "/other")], "");
    });
    let fetcher = HttpDocumentFetcher::new(FetchConfiguration::default()).unwrap();

    let response = fetcher.fetch(server.url("/start")).unwrap();

    assert_eq!(response.status(), 302);
    server.join();
}

#[test]
fn retries_transient_failures_only_up_to_the_configured_limit() {
    let attempts = Arc::new(Mutex::new(0_usize));
    let server_attempts = Arc::clone(&attempts);
    let server = TestServer::start(3, move |mut stream| {
        read_request(&mut stream);
        let mut attempts = server_attempts.lock().unwrap();
        *attempts += 1;
        let status = if *attempts < 3 {
            "503 Service Unavailable"
        } else {
            "200 OK"
        };
        respond(&mut stream, status, &[], "documentation");
    });
    let fetcher = HttpDocumentFetcher::new(FetchConfiguration::new(
        "rust-skgen-test/1.0",
        Duration::from_secs(1),
        2,
    ))
    .unwrap();

    let response = fetcher.fetch(server.url("/guide")).unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(*attempts.lock().unwrap(), 3);
    server.join();
}

#[test]
fn applies_the_configured_request_timeout() {
    let server = TestServer::start(1, move |mut stream| {
        read_request(&mut stream);
        thread::sleep(Duration::from_millis(100));
        respond(&mut stream, "200 OK", &[], "documentation");
    });
    let fetcher = HttpDocumentFetcher::new(FetchConfiguration::new(
        "rust-skgen-test/1.0",
        Duration::from_millis(10),
        0,
    ))
    .unwrap();

    assert!(fetcher.fetch(server.url("/slow")).is_err());
    server.join();
}

#[test]
fn permits_at_most_one_in_flight_request() {
    let active_requests = Arc::new(Mutex::new(0_usize));
    let maximum_active_requests = Arc::new(Mutex::new(0_usize));
    let server_active_requests = Arc::clone(&active_requests);
    let server_maximum_active_requests = Arc::clone(&maximum_active_requests);
    let server = TestServer::start(2, move |mut stream| {
        read_request(&mut stream);
        {
            let mut active_requests = server_active_requests.lock().unwrap();
            *active_requests += 1;
            let mut maximum_active_requests = server_maximum_active_requests.lock().unwrap();
            *maximum_active_requests = (*maximum_active_requests).max(*active_requests);
        }
        thread::sleep(Duration::from_millis(50));
        respond(&mut stream, "200 OK", &[], "documentation");
        *server_active_requests.lock().unwrap() -= 1;
    });
    let fetcher = HttpDocumentFetcher::new(FetchConfiguration::default()).unwrap();
    let start = Arc::new(Barrier::new(3));
    let first_fetcher = fetcher.clone();
    let first_url = server.url("/one");
    let first_start = Arc::clone(&start);
    let first = thread::spawn(move || {
        first_start.wait();
        first_fetcher.fetch(first_url).unwrap();
    });
    let second_fetcher = fetcher.clone();
    let second_url = server.url("/two");
    let second_start = Arc::clone(&start);
    let second = thread::spawn(move || {
        second_start.wait();
        second_fetcher.fetch(second_url).unwrap();
    });

    start.wait();
    first.join().unwrap();
    second.join().unwrap();
    server.join();
    assert_eq!(*maximum_active_requests.lock().unwrap(), 1);
}
