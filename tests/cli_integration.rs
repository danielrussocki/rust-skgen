use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rust_skgen::{
    domain::{ContentFormat, DiscoveryConfiguration, SkillName, SourceUrl},
    metadata::{ManagedSkillMetadata, content_digest},
    storage::TransactionalSkillCreator,
};

struct TemporaryDirectory {
    path: PathBuf,
}

impl TemporaryDirectory {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rust-skgen-cli-{unique}"));
        fs::create_dir(&path).unwrap();
        Self { path }
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn documentation_server() -> (String, thread::JoinHandle<()>) {
    documentation_server_for_requests(2)
}

fn documentation_server_for_requests(requests: usize) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        for _ in 0..requests {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let length = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..length]);
            let body = if request.starts_with("GET /robots.txt ") {
                "User-agent: *\nAllow: /\n"
            } else {
                "<main><p>Local documentation.</p></main>"
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        }
    });
    (format!("http://{address}/start"), handle)
}

fn failing_documentation_server() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        for request_number in 0..4 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let length = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..length]);
            let (status, body) = if request_number == 0 && request.starts_with("GET /robots.txt ") {
                ("200 OK", "User-agent: *\nAllow: /\n")
            } else {
                ("500 Internal Server Error", "unavailable")
            };
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        }
    });
    (format!("http://{address}/start"), handle)
}

fn redirecting_documentation_server() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        for request_number in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let length = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..length]);
            let response = if request_number == 0 && request.starts_with("GET /robots.txt ") {
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 22\r\nConnection: close\r\n\r\nUser-agent: *\nAllow: /\n"
            } else {
                "HTTP/1.1 302 Found\r\nLocation: /moved\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            };
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    (format!("http://{address}/start"), handle)
}

fn mixed_update_server() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        for _ in 0..6 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let length = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..length]);
            let (status, content_type, body) = if request.starts_with("GET /robots.txt ") {
                ("200 OK", "text/plain", "User-agent: *\nAllow: /\n")
            } else if request.starts_with("GET /failing ") {
                ("500 Internal Server Error", "text/html", "unavailable")
            } else {
                (
                    "200 OK",
                    "text/html",
                    "<main><p>Updated documentation.</p></main>",
                )
            };
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        }
    });
    (format!("http://{address}"), handle)
}

fn robots_required_then_missing_server() -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut requests = Vec::new();
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_nonblocking(false).unwrap();
                    let mut request = [0; 1024];
                    let length = stream.read(&mut request).unwrap();
                    let request = String::from_utf8_lossy(&request[..length]);
                    let path = request.split_whitespace().nth(1).unwrap().to_owned();
                    let (status, content_type, body) = match (requests.len(), path.as_str()) {
                        (0, "/robots.txt") => ("200 OK", "text/plain", "User-agent: *\nAllow: /\n"),
                        (1, "/start") => (
                            "200 OK",
                            "text/html",
                            "<main><p>Initial documentation.</p></main>",
                        ),
                        (_, "/robots.txt") => ("404 Not Found", "text/plain", "missing"),
                        (_, "/start") => (
                            "200 OK",
                            "text/html",
                            "<main><p>Unexpected updated documentation.</p></main>",
                        ),
                        _ => ("404 Not Found", "text/plain", "missing"),
                    };
                    requests.push(path);
                    write!(
                        stream,
                        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .unwrap();
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("test server failed to accept a request: {error}"),
            }
        }
        requests
    });
    (format!("http://{address}/start"), handle)
}

fn related_not_found_server() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let mut start_requests = 0;
        for _ in 0..8 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let length = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..length]);
            let (status, content_type, body) = if request.starts_with("GET /robots.txt ") {
                ("200 OK", "text/plain", "User-agent: *\nAllow: /\n")
            } else if request.starts_with("GET /missing ") {
                ("404 Not Found", "text/html", "missing")
            } else {
                start_requests += 1;
                let body = if start_requests == 1 {
                    "<main><p>Initial documentation.</p><a href=\"/missing\">Missing</a></main>"
                } else {
                    "<main><p>Updated documentation.</p><a href=\"/missing\">Missing</a></main>"
                };
                ("200 OK", "text/html", body)
            };
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        }
    });
    (format!("http://{address}/start"), handle)
}

#[test]
fn create_publishes_only_under_the_current_working_directory_skills_root() {
    let working_directory = TemporaryDirectory::new();
    let (source_url, server) = documentation_server();

    let output = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&working_directory.path)
        .args(["create", &source_url, "local-docs"])
        .output()
        .unwrap();
    server.join().unwrap();

    assert!(output.status.success());
    assert!(
        working_directory
            .path
            .join(".agents")
            .join("skills")
            .join("local-docs")
            .is_dir()
    );
    assert!(!working_directory.path.join("local-docs").exists());
}

#[test]
fn create_writes_attributed_content_and_rejects_an_existing_destination() {
    let working_directory = TemporaryDirectory::new();
    let (source_url, server) = documentation_server();

    let first = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&working_directory.path)
        .args(["create", &source_url, "local-docs"])
        .output()
        .unwrap();
    server.join().unwrap();

    assert!(first.status.success());
    let skill = working_directory
        .path
        .join(".agents")
        .join("skills")
        .join("local-docs");
    let content = fs::read_to_string(skill.join("SKILL.md")).unwrap();
    assert!(content.contains(&format!("Source: {source_url}")));

    let (source_url, server) = documentation_server();
    let second = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&working_directory.path)
        .args(["create", &source_url, "local-docs"])
        .output()
        .unwrap();
    server.join().unwrap();

    assert!(!second.status.success());
    assert!(
        String::from_utf8(second.stderr)
            .unwrap()
            .contains("skill destination already exists")
    );
}

#[test]
fn create_does_not_leave_a_partial_directory_when_discovery_fails() {
    let working_directory = TemporaryDirectory::new();
    let (source_url, server) = failing_documentation_server();

    let output = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&working_directory.path)
        .args(["create", &source_url, "failed-docs"])
        .output()
        .unwrap();
    server.join().unwrap();

    assert!(!output.status.success());
    assert!(
        !working_directory
            .path
            .join(".agents")
            .join("skills")
            .join("failed-docs")
            .exists()
    );
}

#[test]
fn create_rejects_invalid_redirected_and_inaccessible_source_urls_without_partial_skills() {
    let invalid_directory = TemporaryDirectory::new();
    let invalid = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&invalid_directory.path)
        .args(["create", "file:///documentation", "invalid-docs"])
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));
    assert!(
        !invalid_directory
            .path
            .join(".agents/skills/invalid-docs")
            .exists()
    );

    let redirected_directory = TemporaryDirectory::new();
    let (redirected_url, server) = redirecting_documentation_server();
    let redirected = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&redirected_directory.path)
        .args(["create", &redirected_url, "redirected-docs"])
        .output()
        .unwrap();
    server.join().unwrap();
    assert_eq!(redirected.status.code(), Some(1));
    assert!(
        !redirected_directory
            .path
            .join(".agents/skills/redirected-docs")
            .exists()
    );

    let inaccessible_directory = TemporaryDirectory::new();
    let inaccessible = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&inaccessible_directory.path)
        .args([
            "create",
            "http://127.0.0.1:1/unavailable",
            "inaccessible-docs",
        ])
        .output()
        .unwrap();
    assert_eq!(inaccessible.status.code(), Some(1));
    assert!(
        !inaccessible_directory
            .path
            .join(".agents/skills/inaccessible-docs")
            .exists()
    );
}

#[test]
fn create_and_update_reject_urls_forbidden_by_access_conditions() {
    let create_directory = TemporaryDirectory::new();
    let restricted_url = "http://user:secret@127.0.0.1:1/restricted";

    let create = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&create_directory.path)
        .args(["create", restricted_url, "restricted-docs"])
        .output()
        .unwrap();

    assert_eq!(create.status.code(), Some(1));
    assert!(
        String::from_utf8(create.stderr)
            .unwrap()
            .contains("crawl policy forbids URL")
    );
    assert!(
        !create_directory
            .path
            .join(".agents/skills/restricted-docs")
            .exists()
    );

    let update_directory = TemporaryDirectory::new();
    let skills_root = update_directory.path.join(".agents/skills");
    let name = SkillName::parse("restricted-docs").unwrap();
    let original_content = "# Existing skill\n";
    let metadata = ManagedSkillMetadata::new(
        SourceUrl::parse(restricted_url).unwrap(),
        DiscoveryConfiguration::default(),
        content_digest(original_content),
    );
    TransactionalSkillCreator::new(&skills_root)
        .create(&name, original_content, &metadata)
        .unwrap();

    let update = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&update_directory.path)
        .args(["update", "restricted-docs"])
        .output()
        .unwrap();

    assert_eq!(update.status.code(), Some(1));
    assert!(
        String::from_utf8(update.stderr)
            .unwrap()
            .contains("crawl policy forbids URL")
    );
    assert_eq!(
        fs::read_to_string(skills_root.join("restricted-docs/SKILL.md")).unwrap(),
        original_content
    );
}

#[test]
fn update_batch_continues_after_discovery_failure_and_exits_with_one() {
    let working_directory = TemporaryDirectory::new();
    let skills_root = working_directory.path.join(".agents/skills");
    let (server_url, server) = mixed_update_server();
    let creator = TransactionalSkillCreator::new(&skills_root);
    let failing = SkillName::parse("failing-docs").unwrap();
    let succeeding = SkillName::parse("succeeding-docs").unwrap();
    let failing_content = "# Failing skill\n";
    let succeeding_content = "# Succeeding skill\n";
    for (name, source_url, content) in [
        (&failing, format!("{server_url}/failing"), failing_content),
        (
            &succeeding,
            format!("{server_url}/succeeding"),
            succeeding_content,
        ),
    ] {
        let metadata = ManagedSkillMetadata::new(
            SourceUrl::parse(&source_url).unwrap(),
            DiscoveryConfiguration::default(),
            content_digest(content),
        );
        creator.create(name, content, &metadata).unwrap();
    }

    let output = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&working_directory.path)
        .args(["update", "failing-docs", "succeeding-docs"])
        .output()
        .unwrap();
    server.join().unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("Failed skill: failing-docs")
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("Updated skill: succeeding-docs")
    );
    assert_eq!(
        fs::read_to_string(skills_root.join("failing-docs/SKILL.md")).unwrap(),
        failing_content
    );
    assert_ne!(
        fs::read_to_string(skills_root.join("succeeding-docs/SKILL.md")).unwrap(),
        succeeding_content
    );
}

#[test]
fn update_handles_all_skills_mixed_selections_and_individual_changes() {
    let working_directory = TemporaryDirectory::new();
    let skills_root = working_directory.path.join(".agents").join("skills");
    let current_name = SkillName::parse("managed-docs").unwrap();
    let initial_content = "# Previous skill\n";
    let (source_url, server) = documentation_server_for_requests(6);
    let metadata = ManagedSkillMetadata::new(
        SourceUrl::parse(&source_url).unwrap(),
        DiscoveryConfiguration::default(),
        content_digest(initial_content),
    );
    TransactionalSkillCreator::new(&skills_root)
        .create(&current_name, initial_content, &metadata)
        .unwrap();

    let all = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&working_directory.path)
        .arg("update")
        .output()
        .unwrap();
    assert!(all.status.success());
    assert!(
        String::from_utf8(all.stdout)
            .unwrap()
            .contains("Updated skill: managed-docs")
    );

    let mixed = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&working_directory.path)
        .args(["update", "managed-docs", "missing-docs"])
        .output()
        .unwrap();
    assert!(!mixed.status.success());
    assert!(
        String::from_utf8(mixed.stderr)
            .unwrap()
            .contains("Failed skill: missing-docs")
    );

    let configured = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&working_directory.path)
        .args([
            "update",
            "managed-docs",
            "--name",
            "renamed-docs",
            "--source-url",
            &source_url,
            "--format",
            "organized-content",
        ])
        .output()
        .unwrap();
    server.join().unwrap();

    assert!(configured.status.success());
    assert!(
        String::from_utf8(configured.stdout)
            .unwrap()
            .contains("Updated skill: renamed-docs")
    );
    let metadata = ManagedSkillMetadata::from_json(
        &fs::read_to_string(skills_root.join("renamed-docs").join("metadata.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(metadata.source_url().as_url().as_str(), source_url);
    assert_eq!(
        metadata.discovery().content_format(),
        ContentFormat::OrganizedContent
    );
}

#[test]
fn update_rebuild_confirmation_accepts_y_and_preserves_skills_for_n_or_eof() {
    let accepted_directory = TemporaryDirectory::new();
    let (source_url, server) = documentation_server();
    create_skill_without_metadata(&accepted_directory.path, "accepted-docs");
    let mut accepted = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&accepted_directory.path)
        .args(["update", "accepted-docs", "--source-url", &source_url])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut accepted_input = accepted.stdin.take().unwrap();
    accepted_input.write_all(b"y\n").unwrap();
    drop(accepted_input);
    let accepted = accepted.wait_with_output().unwrap();
    server.join().unwrap();
    assert!(accepted.status.success());
    assert!(
        accepted_directory
            .path
            .join(".agents/skills/accepted-docs/metadata.json")
            .is_file()
    );

    for response in [Some(b"n\n".as_slice()), None] {
        let rejected_directory = TemporaryDirectory::new();
        create_skill_without_metadata(&rejected_directory.path, "rejected-docs");
        let mut command = Command::new(env!("CARGO_BIN_EXE_rust-skgen"));
        command
            .current_dir(&rejected_directory.path)
            .args([
                "update",
                "rejected-docs",
                "--source-url",
                "http://127.0.0.1:1/unreachable",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().unwrap();
        if let Some(response) = response {
            child.stdin.as_mut().unwrap().write_all(response).unwrap();
        }
        drop(child.stdin.take());
        let output = child.wait_with_output().unwrap();

        assert!(!output.status.success());
        assert_eq!(
            fs::read_to_string(
                rejected_directory
                    .path
                    .join(".agents/skills/rejected-docs/SKILL.md")
            )
            .unwrap(),
            "# Existing skill\n"
        );
        assert!(
            !rejected_directory
                .path
                .join(".agents/skills/rejected-docs/metadata.json")
                .exists()
        );
    }
}

#[test]
fn update_without_changes_confirms_a_mismatched_content_digest_for_a_selected_skill() {
    let accepted_directory = TemporaryDirectory::new();
    let (source_url, server) = documentation_server();
    create_manually_modified_managed_skill(&accepted_directory.path, "accepted-docs", &source_url);

    let mut accepted = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&accepted_directory.path)
        .args(["update", "accepted-docs"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    accepted.stdin.as_mut().unwrap().write_all(b"y\n").unwrap();
    drop(accepted.stdin.take());
    let accepted = accepted.wait_with_output().unwrap();

    assert_eq!(accepted.status.code(), Some(0));
    assert!(
        String::from_utf8(accepted.stderr)
            .unwrap()
            .contains("Rebuild metadata and update this skill? [y/N]")
    );
    assert!(
        String::from_utf8(accepted.stdout)
            .unwrap()
            .contains("Updated skill: accepted-docs")
    );
    assert_ne!(
        fs::read_to_string(
            accepted_directory
                .path
                .join(".agents/skills/accepted-docs/SKILL.md")
        )
        .unwrap(),
        "# Manually changed skill\n"
    );
    server.join().unwrap();

    for response in [Some(b"n\n".as_slice()), None] {
        let rejected_directory = TemporaryDirectory::new();
        create_manually_modified_managed_skill(
            &rejected_directory.path,
            "rejected-docs",
            "http://127.0.0.1:1/unreachable",
        );
        let mut command = Command::new(env!("CARGO_BIN_EXE_rust-skgen"));
        command
            .current_dir(&rejected_directory.path)
            .args(["update", "rejected-docs"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut rejected = command.spawn().unwrap();
        if let Some(response) = response {
            rejected
                .stdin
                .as_mut()
                .unwrap()
                .write_all(response)
                .unwrap();
        }
        drop(rejected.stdin.take());
        let rejected = rejected.wait_with_output().unwrap();

        assert_eq!(rejected.status.code(), Some(1));
        assert!(
            String::from_utf8(rejected.stderr)
                .unwrap()
                .contains("Rebuild metadata and update this skill? [y/N]")
        );
        assert_eq!(
            fs::read_to_string(
                rejected_directory
                    .path
                    .join(".agents/skills/rejected-docs/SKILL.md")
            )
            .unwrap(),
            "# Manually changed skill\n"
        );
    }
}

#[test]
fn update_without_names_confirms_each_mismatched_content_digest() {
    let working_directory = TemporaryDirectory::new();
    let (source_url, server) = documentation_server();
    create_manually_modified_managed_skill(&working_directory.path, "accepted-docs", &source_url);
    create_manually_modified_managed_skill(
        &working_directory.path,
        "rejected-docs",
        "http://127.0.0.1:1/unreachable",
    );

    let mut update = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&working_directory.path)
        .arg("update")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    update.stdin.as_mut().unwrap().write_all(b"y\nn\n").unwrap();
    drop(update.stdin.take());
    let update = update.wait_with_output().unwrap();

    assert_eq!(update.status.code(), Some(1));
    let standard_error = String::from_utf8(update.stderr).unwrap();
    assert_eq!(
        standard_error
            .matches("Rebuild metadata and update this skill? [y/N]")
            .count(),
        2
    );
    assert!(standard_error.contains("Failed skill: rejected-docs"));
    assert!(
        String::from_utf8(update.stdout)
            .unwrap()
            .contains("Updated skill: accepted-docs")
    );
    assert_ne!(
        fs::read_to_string(
            working_directory
                .path
                .join(".agents/skills/accepted-docs/SKILL.md")
        )
        .unwrap(),
        "# Manually changed skill\n"
    );
    assert_eq!(
        fs::read_to_string(
            working_directory
                .path
                .join(".agents/skills/rejected-docs/SKILL.md")
        )
        .unwrap(),
        "# Manually changed skill\n"
    );
    server.join().unwrap();
}

#[test]
fn update_without_changes_preserves_a_persisted_robots_requirement() {
    let working_directory = TemporaryDirectory::new();
    let (source_url, server) = robots_required_then_missing_server();

    let created = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&working_directory.path)
        .args([
            "create",
            &source_url,
            "robots-required-docs",
            "--require-robots-txt",
            "true",
        ])
        .output()
        .unwrap();
    assert!(
        created.status.success(),
        "create failed: {}",
        String::from_utf8_lossy(&created.stderr)
    );
    let skill_path = working_directory
        .path
        .join(".agents/skills/robots-required-docs/SKILL.md");
    let previous_content = fs::read_to_string(&skill_path).unwrap();

    let updated = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&working_directory.path)
        .args(["update", "robots-required-docs"])
        .output()
        .unwrap();
    let requests = server.join().unwrap();

    assert_eq!(updated.status.code(), Some(1));
    assert!(
        String::from_utf8(updated.stderr)
            .unwrap()
            .contains("robots.txt returned unexpected HTTP status 404")
    );
    assert_eq!(fs::read_to_string(skill_path).unwrap(), previous_content);
    assert_eq!(requests, vec!["/robots.txt", "/start", "/robots.txt"]);
}

#[test]
fn create_and_update_publish_unavailable_related_documentation_after_http_not_found() {
    let working_directory = TemporaryDirectory::new();
    let (source_url, server) = related_not_found_server();
    let skill_path = working_directory
        .path
        .join(".agents/skills/missing-related-docs/SKILL.md");

    let created = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&working_directory.path)
        .args(["create", &source_url, "missing-related-docs"])
        .output()
        .unwrap();
    assert!(created.status.success());
    let initial_content = fs::read_to_string(&skill_path).unwrap();
    assert!(initial_content.contains("Initial documentation."));
    assert!(initial_content.contains("Documentation is unavailable for this source."));
    assert!(initial_content.contains(&format!(
        "{}/missing",
        source_url.trim_end_matches("/start")
    )));

    let updated = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&working_directory.path)
        .args(["update", "missing-related-docs"])
        .output()
        .unwrap();
    server.join().unwrap();

    assert!(updated.status.success());
    let updated_content = fs::read_to_string(skill_path).unwrap();
    assert!(updated_content.contains("Updated documentation."));
    assert!(updated_content.contains("Documentation is unavailable for this source."));
}

#[test]
fn executable_uses_specified_exit_codes_and_output_channels() {
    let working_directory = TemporaryDirectory::new();

    let success = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&working_directory.path)
        .arg("update")
        .output()
        .unwrap();

    assert_eq!(success.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(success.stdout).unwrap(),
        "No managed skills found.\n"
    );
    assert!(success.stderr.is_empty());

    let skill_failure = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&working_directory.path)
        .args(["update", "missing-docs"])
        .output()
        .unwrap();

    assert_eq!(skill_failure.status.code(), Some(1));
    assert!(skill_failure.stdout.is_empty());
    assert!(
        String::from_utf8(skill_failure.stderr)
            .unwrap()
            .starts_with("Failed skill: missing-docs:")
    );

    let invalid_arguments = Command::new(env!("CARGO_BIN_EXE_rust-skgen"))
        .current_dir(&working_directory.path)
        .args(["update", "--source-url", "https://example.com/docs"])
        .output()
        .unwrap();

    assert_eq!(invalid_arguments.status.code(), Some(2));
    assert!(invalid_arguments.stdout.is_empty());
    assert!(
        String::from_utf8(invalid_arguments.stderr)
            .unwrap()
            .contains("Invalid argument: configuration changes require exactly one selected skill")
    );
}

fn create_skill_without_metadata(working_directory: &std::path::Path, name: &str) {
    let skill = working_directory.join(".agents").join("skills").join(name);
    fs::create_dir_all(&skill).unwrap();
    fs::write(skill.join("SKILL.md"), "# Existing skill\n").unwrap();
}

fn create_manually_modified_managed_skill(
    working_directory: &std::path::Path,
    name: &str,
    source_url: &str,
) {
    let skills_root = working_directory.join(".agents/skills");
    let name = SkillName::parse(name).unwrap();
    let original_content = "# Generated skill\n";
    let metadata = ManagedSkillMetadata::new(
        SourceUrl::parse(source_url).unwrap(),
        DiscoveryConfiguration::default(),
        content_digest(original_content),
    );
    TransactionalSkillCreator::new(&skills_root)
        .create(&name, original_content, &metadata)
        .unwrap();
    fs::write(
        skills_root.join(name.as_str()).join("SKILL.md"),
        "# Manually changed skill\n",
    )
    .unwrap();
}
