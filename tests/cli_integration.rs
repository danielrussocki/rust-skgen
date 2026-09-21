use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{SystemTime, UNIX_EPOCH},
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

fn create_skill_without_metadata(working_directory: &std::path::Path, name: &str) {
    let skill = working_directory.join(".agents").join("skills").join(name);
    fs::create_dir_all(&skill).unwrap();
    fs::write(skill.join("SKILL.md"), "# Existing skill\n").unwrap();
}
