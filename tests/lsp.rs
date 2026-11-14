//! `ortho lsp`, driven over stdio as an editor drives it.
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

mod common;
use common::repository;

/// An empty folder of the test's own.
fn scratch(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("lsp").join(name);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

/// `file:///home/a/m.sysml`, or `file:///C:/a/m.sysml` on Windows.
fn uri(path: &Path) -> String {
    let path = path.to_str().unwrap().replace('\\', "/").replace(' ', "%20");
    if path.starts_with('/') { format!("file://{path}") } else { format!("file:///{path}") }
}

fn frame(message: &Value) -> String {
    let body = message.to_string();
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

/// Starts a session in `root`, sends `messages` after `initialize`, shuts down, and
/// returns everything the server sent, with how it exited.
fn session(root: &Path, messages: &[Value]) -> (Vec<Value>, bool) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ortho"))
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = String::new();
    let initialize = json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize",
        "params": { "rootUri": uri(root), "capabilities": {} } });
    input.push_str(&frame(&initialize));
    input.push_str(&frame(&json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} })));
    for message in messages {
        input.push_str(&frame(message));
    }
    input.push_str(&frame(&json!({ "jsonrpc": "2.0", "id": 999, "method": "shutdown" })));
    input.push_str(&frame(&json!({ "jsonrpc": "2.0", "method": "exit" })));
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut sent = Vec::new();
    loop {
        let mut header = String::new();
        if output.read_line(&mut header).unwrap() == 0 {
            break;
        }
        let length: usize = header.trim().strip_prefix("Content-Length: ").unwrap().parse().unwrap();
        output.read_line(&mut String::new()).unwrap();
        let mut body = vec![0; length];
        output.read_exact(&mut body).unwrap();
        sent.push(serde_json::from_slice(&body).unwrap());
    }
    (sent, child.wait().unwrap().success())
}

fn open(path: &Path, text: &str) -> Value {
    json!({ "jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
        "textDocument": { "uri": uri(path), "languageId": "sysml", "version": 1, "text": text } } })
}

fn change(path: &Path, text: &str) -> Value {
    json!({ "jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
        "textDocument": { "uri": uri(path), "version": 2 }, "contentChanges": [{ "text": text }] } })
}

fn save(path: &Path) -> Value {
    json!({ "jsonrpc": "2.0", "method": "textDocument/didSave", "params": { "textDocument": { "uri": uri(path) } } })
}

fn close(path: &Path) -> Value {
    json!({ "jsonrpc": "2.0", "method": "textDocument/didClose", "params": { "textDocument": { "uri": uri(path) } } })
}

fn request(id: u64, method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
}

fn at(path: &Path, line: u64, character: u64) -> Value {
    json!({ "textDocument": { "uri": uri(path) }, "position": { "line": line, "character": character } })
}

fn response(sent: &[Value], id: u64) -> &Value {
    &sent.iter().find(|message| message["id"] == id && message.get("method").is_none()).unwrap()["result"]
}

/// What the server last reported for a file.
fn diagnostics(sent: &[Value], path: &Path) -> Vec<(u64, u64, String)> {
    let mut reports = sent.iter().filter(|message| {
        message["method"] == "textDocument/publishDiagnostics" && message["params"]["uri"] == uri(path)
    });
    let last = reports.next_back().unwrap_or_else(|| panic!("nothing reported for {}", path.display()));
    let found = last["params"]["diagnostics"].as_array().unwrap().iter();
    found
        .map(|d| {
            let line = d["range"]["start"]["line"].as_u64().unwrap();
            (d["severity"].as_u64().unwrap(), line, d["message"].as_str().unwrap().to_string())
        })
        .collect()
}

/// A copy of an example for a test of its own, to write into.
fn example(name: &str, test: &str) -> PathBuf {
    let folder = scratch(&format!("{test}/{name}"));
    let source = repository().join("examples").join(name);
    for entry in std::fs::read_dir(&source).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "sysml") {
            std::fs::copy(&path, folder.join(path.file_name().unwrap())).unwrap();
        }
    }
    folder
}

#[test]
fn starts_and_shuts_down() {
    let folder = scratch("starts");
    let (sent, clean) = session(&folder, &[]);
    assert!(clean);
    let capabilities = &response(&sent, 0)["capabilities"];
    assert_eq!(capabilities["definitionProvider"], true);
    assert_eq!(response(&sent, 0)["serverInfo"]["name"], "ortho");
}

#[test]
fn reports_the_problems_of_every_file_in_the_folder_as_typed() {
    let folder = example("coffee-machine", "problems");
    let logical = folder.join("logical.sysml");
    let text = std::fs::read_to_string(&logical).unwrap();
    // Renamed as typed, unsaved: the files that perform it no longer resolve.
    let renamed = text.replace("action grindBeans;", "action millBeans;");
    let (sent, _) = session(&folder, &[open(&logical, &text), change(&logical, &renamed)]);
    let physical = diagnostics(&sent, &folder.join("physical.sysml"));
    assert!(
        physical.iter().any(|(severity, _, message)| *severity == 1 && message.contains("grindBeans")),
        "{physical:?}"
    );
    assert_eq!(diagnostics(&sent, &folder.join("use-case.sysml")), vec![]);
}

#[test]
fn reports_a_syntax_error_where_it_is() {
    let folder = scratch("syntax");
    let model = folder.join("model.sysml");
    let (sent, _) = session(&folder, &[open(&model, "package P {\n    part def X Y;\n}\n")]);
    let found = diagnostics(&sent, &model);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].0, found[0].1), (1, 1));
}

#[test]
fn goes_to_what_a_name_names_in_another_file() {
    let folder = example("coffee-machine", "definition");
    let physical = folder.join("physical.sysml");
    let text = std::fs::read_to_string(&physical).unwrap();
    let (line, column) = text
        .lines()
        .enumerate()
        .find_map(|(i, line)| line.find("grindBeans").filter(|_| line.contains("perform")).map(|c| (i, c)))
        .unwrap();
    let (sent, _) = session(
        &folder,
        &[open(&physical, &text), request(1, "textDocument/definition", at(&physical, line as u64, column as u64 + 2))],
    );
    let location = response(&sent, 1);
    assert_eq!(location["uri"], uri(&folder.join("logical.sysml")));
    let logical = std::fs::read_to_string(folder.join("logical.sysml")).unwrap();
    let declared = logical.lines().position(|line| line.trim() == "action grindBeans;").unwrap();
    assert_eq!(location["range"]["start"]["line"], declared as u64);
    let start = location["range"]["start"]["character"].as_u64().unwrap() as usize;
    assert_eq!(&logical.lines().nth(declared).unwrap()[start..start + 10], "grindBeans");
}

/// A session kept open, to talk with the server as an editor does while its preview
/// is asked as a browser asks it.
struct Live {
    child: std::process::Child,
    input: std::process::ChildStdin,
    sent: std::sync::mpsc::Receiver<Value>,
}

impl Live {
    /// Starts a session in `root`, as an editor that opens what it is asked to.
    fn start(root: &Path) -> Live {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ortho"))
            .arg("lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let (sender, sent) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            loop {
                let mut header = String::new();
                if output.read_line(&mut header).unwrap_or(0) == 0 {
                    break;
                }
                let length: usize = header.trim().strip_prefix("Content-Length: ").unwrap().parse().unwrap();
                output.read_line(&mut String::new()).unwrap();
                let mut body = vec![0; length];
                output.read_exact(&mut body).unwrap();
                if sender.send(serde_json::from_slice(&body).unwrap()).is_err() {
                    break;
                }
            }
        });
        let mut live = Live { input: child.stdin.take().unwrap(), child, sent };
        let capabilities = json!({ "window": { "showDocument": { "support": true } } });
        live.send(&request(0, "initialize", json!({ "rootUri": uri(root), "capabilities": capabilities })));
        live.send(&json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }));
        live
    }

    fn send(&mut self, message: &Value) {
        self.input.write_all(frame(message).as_bytes()).unwrap();
    }

    /// The next thing the server asks the editor to show.
    fn shown(&mut self) -> Value {
        loop {
            let message = self.sent.recv_timeout(std::time::Duration::from_secs(20)).expect("nothing shown");
            if message["method"] == "window/showDocument" {
                return message["params"].clone();
            }
        }
    }

    /// Whether the server asks the editor to show nothing for a while.
    fn shows_nothing(&mut self, within: std::time::Duration) -> bool {
        let until = std::time::Instant::now() + within;
        while let Some(left) = until.checked_duration_since(std::time::Instant::now()) {
            match self.sent.recv_timeout(left) {
                Ok(message) if message["method"] == "window/showDocument" => return false,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        true
    }

    /// Opens a model file, and gives the port of the preview the server opens for it.
    fn preview(&mut self, file: &Path) -> u16 {
        self.send(&open(file, &std::fs::read_to_string(file).unwrap()));
        let shown = self.shown();
        assert_eq!(shown["external"], true);
        let url = shown["uri"].as_str().unwrap();
        url.strip_prefix("http://127.0.0.1:").and_then(|rest| rest.strip_suffix('/')).unwrap().parse().unwrap()
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Asks the preview as a browser does: the status and the body of its answer.
fn http(port: u16, method: &str, path: &str, headers: &str, body: &str) -> (u16, String) {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(stream, "{method} {path} HTTP/1.1\r\n{headers}Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).unwrap();
    let (head, body) = answer.split_once("\r\n\r\n").unwrap();
    (head.split(' ').nth(1).unwrap().parse().unwrap(), body.to_string())
}

/// A page left open in the browser: it asks the preview for each newer render, as the
/// page does, until the test ends.
fn page(port: u16) {
    std::thread::spawn(move || {
        let (mut session, mut version) = (String::new(), Value::from(0));
        loop {
            let Ok(mut stream) = std::net::TcpStream::connect(("127.0.0.1", port)) else { return };
            let path = format!("/state?session={session}&after={version}");
            if write!(stream, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n").is_err() {
                return;
            }
            let mut answer = String::new();
            let read = stream.read_to_string(&mut answer);
            let Some(state) = read.ok().and_then(|_| answer.split_once("\r\n\r\n")).map(|(_, body)| body.to_string())
            else {
                return;
            };
            let Ok(state) = serde_json::from_str::<Value>(&state) else { return };
            session = state["session"].as_str().unwrap_or("").to_string();
            version = state["version"].clone();
        }
    });
    // Waiting, as a page open in the browser is.
    std::thread::sleep(std::time::Duration::from_millis(500));
}

/// What the preview shows now, without waiting for anything newer.
fn now(port: u16) -> Value {
    get(port, "/state?session=&after=0")
}

/// What the preview shows once it is as expected, since the server reads what the
/// editor sends in its own time.
fn shows(port: u16, expected: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..100 {
        let state = now(port);
        if expected(&state) {
            return state;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("the preview shows {}", now(port)["model"]);
}

fn model_shown(port: u16, model: &str) -> Value {
    shows(port, |state| state["model"] == model && state["closed"] == false)
}

fn get(port: u16, path: &str) -> Value {
    let (status, body) = http(port, "GET", path, &format!("Host: 127.0.0.1:{port}\r\n"), "");
    assert_eq!(status, 200, "{body}");
    serde_json::from_str(&body).unwrap()
}

/// A post from the preview's own page.
fn post(port: u16, path: &str, body: Value) -> Value {
    let headers = format!("Host: 127.0.0.1:{port}\r\nOrigin: http://127.0.0.1:{port}\r\n");
    let (status, body) = http(port, "POST", path, &headers, &body.to_string());
    assert_eq!(status, 200, "{body}");
    serde_json::from_str(&body).unwrap()
}

/// What `ortho render` run at the project's root, where the server runs, writes.
fn rendered(folder: &Path) -> Vec<(String, String)> {
    let done =
        Command::new(env!("CARGO_BIN_EXE_ortho")).args(["render", ".", "-o", "expected"]).current_dir(folder).output();
    assert!(done.unwrap().status.success());
    let files = std::fs::read_dir(folder.join("expected")).unwrap().map(|entry| entry.unwrap());
    files.map(|file| (file.file_name().into_string().unwrap(), std::fs::read_to_string(file.path()).unwrap())).collect()
}

#[test]
fn opens_a_preview_that_shows_the_diagrams_ortho_render_writes_and_follows_each_save() {
    let folder = example("coffee-machine", "preview");
    let logical = folder.join("logical.sysml");
    let mut live = Live::start(&folder);
    let port = live.preview(&logical);
    let (status, page) = http(port, "GET", "/", &format!("Host: localhost:{port}\r\n"), "");
    assert!(status == 200 && page.contains("<title>Ortho preview</title>"));
    let state = get(port, "/state?session=&after=0");
    assert_eq!(state["model"], "coffee-machine");
    assert_eq!(state["errors"], json!([]));
    let expected = rendered(&folder);
    let views = state["views"].as_array().unwrap();
    assert_eq!((views.len(), expected.len()), (6, 6));
    for (file, svg) in &expected {
        let view = views.iter().find(|view| view["file"] == file.as_str()).unwrap();
        assert_eq!(view["svg"], svg.as_str(), "{file}");
    }
    assert_eq!(views[0]["heading"], "Use case view — Coffee Machine");
    // Nothing is written beside the model until the page is asked to.
    assert!(!folder.join("diagrams").exists());

    // Saved with a mistake: the page is told where, and keeps the diagrams it has.
    let text = std::fs::read_to_string(&logical).unwrap();
    let line = text.lines().position(|line| line.trim() == "action grindBeans;").unwrap();
    live.send(&change(&logical, &text.replace("action grindBeans;", "action grindBeans")));
    live.send(&save(&logical));
    let after = format!("/state?session={}&after={}", state["session"].as_str().unwrap(), state["version"]);
    let broken = get(port, &after);
    assert_eq!(broken["views"], json!([]));
    assert_eq!(broken["folder"], state["folder"]);
    assert_eq!(broken["errors"][0]["file"], "logical.sysml");
    assert!(broken["errors"][0]["line"].as_u64().unwrap() > line as u64);
}

#[test]
fn a_box_of_the_preview_opens_its_line_in_the_editor_and_save_writes_the_diagrams() {
    let folder = example("coffee-machine", "page");
    let logical = folder.join("logical.sysml");
    let mut live = Live::start(&folder);
    let port = live.preview(&logical);
    assert_eq!(post(port, "/open", json!({ "file": "logical.sysml", "line": 7 })), json!({ "ok": true }));
    let shown = live.shown();
    assert_eq!(shown["uri"], uri(&logical));
    assert_eq!(shown["selection"]["start"], json!({ "line": 6, "character": 0 }));
    // Only the model's own files, and only for the preview's own page.
    std::fs::write(folder.parent().unwrap().join("other.sysml"), "").unwrap();
    assert_eq!(post(port, "/open", json!({ "file": "../other.sysml", "line": 1 })), json!({ "ok": false }));
    let ours = format!("Host: 127.0.0.1:{port}\r\n");
    let body = json!({ "file": "logical.sysml", "line": 7 }).to_string();
    assert_eq!(http(port, "POST", "/open", &ours, &body).0, 403);
    let elsewhere = format!("{ours}Origin: http://example.com\r\n");
    assert_eq!(http(port, "POST", "/save", &elsewhere, "{}").0, 403);
    assert_eq!(http(port, "GET", "/state", "Host: example.com\r\n", "").0, 403);
    assert!(!folder.join("diagrams").exists());

    let saved = post(port, "/save", json!({}));
    assert_eq!(saved["ok"], true, "{saved}");
    for (file, svg) in rendered(&folder) {
        assert_eq!(std::fs::read_to_string(folder.join("diagrams").join(&file)).unwrap(), svg, "{file}");
    }
}

#[test]
fn a_code_action_on_a_view_shows_it_in_the_preview() {
    let folder = example("coffee-machine", "action");
    let logical = folder.join("logical.sysml");
    let text = std::fs::read_to_string(&logical).unwrap();
    let line = text.lines().position(|line| line.trim_start().starts_with("view ")).unwrap() as u64;
    let range = json!({ "start": { "line": line, "character": 0 }, "end": { "line": line, "character": 0 } });
    let params = json!({ "textDocument": { "uri": uri(&logical) }, "range": range, "context": { "diagnostics": [] } });
    let mut elsewhere = params.clone();
    elsewhere["range"]["start"]["line"] = json!(0);
    elsewhere["range"]["end"]["line"] = json!(0);
    let asked = [
        open(&logical, &text),
        request(1, "textDocument/codeAction", params),
        request(2, "textDocument/codeAction", elsewhere),
    ];
    let (sent, _) = session(&folder, &asked);
    let actions = response(&sent, 1).as_array().unwrap();
    assert_eq!(actions.len(), 1, "{actions:?}");
    assert_eq!(actions[0]["title"], "Show 'Logical view — Coffee Machine' in the preview");
    assert_eq!(response(&sent, 2), &json!([]));

    // With no page open, the browser is opened at the view; with one, it turns to it.
    let command = &actions[0]["command"];
    let execute = json!({ "command": command["command"], "arguments": command["arguments"] });
    let mut live = Live::start(&folder);
    live.send(&request(3, "workspace/executeCommand", execute));
    let shown = live.shown();
    assert_eq!(shown["external"], true);
    let url = shown["uri"].as_str().unwrap();
    let (address, view) = url.split_once("/#").unwrap();
    assert_eq!(view, "logical-view-coffee-machine.svg");
    let port = address.rsplit(':').next().unwrap().parse().unwrap();
    let state = get(port, "/state?session=&after=0");
    assert_eq!(state["select"]["file"], view);
    assert_eq!(state["views"].as_array().unwrap().len(), 6);
}

/// Two models in folders of their own, side by side in one project, as a test's own.
fn two_models(test: &str) -> (PathBuf, PathBuf, PathBuf) {
    let coffee = example("coffee-machine", test);
    let drone = example("survey-drone", test);
    let root = coffee.parent().unwrap().to_path_buf();
    (root, coffee.join("logical.sysml"), drone.join("logical.sysml"))
}

#[test]
fn opens_the_preview_again_for_a_model_file_opened_once_the_page_is_closed() {
    let (root, coffee, drone) = two_models("reopen");
    let mut live = Live::start(&root);
    let port = live.preview(&coffee);
    // A page that opens and is closed again, by its user or by itself.
    let state = now(port);
    let mut held = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    let path = format!("/state?session={}&after={}", state["session"].as_str().unwrap(), state["version"]);
    write!(held, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));
    drop(held);
    std::thread::sleep(std::time::Duration::from_millis(1500));

    live.send(&open(&drone, &std::fs::read_to_string(&drone).unwrap()));
    let shown = live.shown();
    assert_eq!(shown["external"], true);
    assert_eq!(shown["uri"], format!("http://127.0.0.1:{port}/"));
    model_shown(port, "survey-drone");
}

#[test]
fn with_a_page_open_the_preview_turns_to_the_model_opened_or_edited_and_opens_no_second() {
    let (root, coffee, drone) = two_models("follow");
    let mut live = Live::start(&root);
    let port = live.preview(&coffee);
    page(port);

    live.send(&open(&drone, &std::fs::read_to_string(&drone).unwrap()));
    assert!(live.shows_nothing(std::time::Duration::from_secs(3)), "a second page was opened");
    model_shown(port, "survey-drone");

    // Edited, unsaved, the first model is drawn again: it is the one worked on.
    let text = std::fs::read_to_string(&coffee).unwrap();
    live.send(&change(&coffee, &format!("{text}\n")));
    model_shown(port, "coffee-machine");
    assert!(live.shows_nothing(std::time::Duration::from_secs(2)), "a second page was opened");
}

#[test]
fn the_preview_turns_to_a_model_still_open_when_one_is_closed_and_closes_with_the_last() {
    let (root, coffee, drone) = two_models("close");
    let other = coffee.with_file_name("physical.sysml");
    let mut live = Live::start(&root);
    let port = live.preview(&coffee);
    live.send(&open(&other, &std::fs::read_to_string(&other).unwrap()));
    live.send(&open(&drone, &std::fs::read_to_string(&drone).unwrap()));
    let opened = model_shown(port, "survey-drone");

    // The drone's only file closed: the coffee machine, still open, is shown.
    live.send(&close(&drone));
    model_shown(port, "coffee-machine");
    // One of its two files closed: it stays, and is not drawn again.
    let version = now(port)["version"].clone();
    live.send(&close(&coffee));
    // The last closed: the page is told to close.
    live.send(&close(&other));
    let closed = shows(port, |state| state["closed"] == true);
    assert_eq!(closed["version"], version.as_u64().unwrap() + 1);
    assert!(opened["version"].as_u64() < version.as_u64());

    // Opened again, it shows the model, and opens the browser for it.
    live.send(&open(&drone, &std::fs::read_to_string(&drone).unwrap()));
    assert_eq!(live.shown()["uri"], format!("http://127.0.0.1:{port}/"));
    model_shown(port, "survey-drone");
}
