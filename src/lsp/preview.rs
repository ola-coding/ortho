//! The live preview of `ortho lsp`: a page in the browser, served on this machine
//! only, that shows the diagrams and the table of requirements of the model being
//! edited, and draws them again as it is typed. An editor that has no panel for a
//! language server to draw in, such as Zed, opens the page instead. The page waits on
//! `state` for each newer render, and posts back to open a line in the editor or to
//! save the views.
use super::Editor;
use super::protocol::path_to_uri;
use crate::pipeline::relative_to_working_folder;
use serde_json::{Value, json};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

const PAGE: &str = include_str!("preview.html");
/// The page's reader of a table of requirements, a copy of the VS Code extension's.
const MARKDOWN: &str = include_str!("markdown.js");

/// How long a page's request for a newer render is held before it is answered with
/// the same one, to ask again.
const HOLD: Duration = Duration::from_secs(25);

/// How often a held request looks whether its page is still there.
const CHECK: Duration = Duration::from_millis(250);

/// What the page shows of a model.
pub struct Shown {
    /// The model's folder.
    pub folder: PathBuf,
    /// The views, each as its file name, heading, summary and SVG, or Markdown for a
    /// table of requirements; none while the model does not load.
    pub views: Vec<[String; 4]>,
    /// Where each requirement a table lists is declared, for its box to open.
    pub requirements: Value,
    pub errors: Vec<Note>,
    pub notes: Vec<Note>,
}

/// Something ortho reported, and the file, named from the working folder, and line,
/// from 1, it is about, if it is about one.
pub struct Note {
    pub place: Option<(String, usize)>,
    pub message: String,
}

impl Note {
    fn json(&self) -> Value {
        match &self.place {
            Some((file, line)) => json!({ "file": file, "line": line, "message": self.message }),
            None => json!({ "message": self.message }),
        }
    }
}

#[derive(Default)]
struct State {
    /// Counts what is shown: a page asks for what comes after the one it has.
    version: u64,
    shown: Option<Shown>,
    /// The view the editor last asked the page to turn to, and which time that was.
    select: Option<(String, u64)>,
    /// The folder, in the model's, that the page's save writes the diagrams to.
    save_to: String,
    /// How many pages are waiting for a newer render, and when one last asked.
    waiting: usize,
    asked: Option<Instant>,
    /// Whether the editor has closed every model file, for the page to close too.
    closed: bool,
}

impl State {
    /// Whether a page is open: one is waiting, or was answered a moment ago and is
    /// about to ask again.
    fn watched(&self) -> bool {
        self.waiting > 0 || self.asked.is_some_and(|asked| asked.elapsed() < Duration::from_secs(1))
    }
}

struct Shared {
    state: Mutex<State>,
    changed: Condvar,
    /// Tells this server from one before it at the same address, whose versions
    /// counted from zero too.
    session: String,
    port: u16,
    editor: Editor,
}

/// The preview's server, running until the process ends.
#[derive(Clone)]
pub struct Preview {
    shared: Arc<Shared>,
}

impl Preview {
    /// Starts serving the page for the project at `root`. The port is chosen from the
    /// project's path, so a page left open finds the server again after the editor
    /// is started again; the next free one is taken if another server has it.
    pub fn start(root: &Path, save_to: &str, editor: Editor) -> io::Result<Preview> {
        let first = 47000 + (hash(root.to_string_lossy().as_bytes()) % 1000) as u16;
        let listener = (first..first + 20)
            .find_map(|port| TcpListener::bind((Ipv4Addr::LOCALHOST, port)).ok())
            .map_or_else(|| TcpListener::bind((Ipv4Addr::LOCALHOST, 0)), Ok)?;
        let since = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let shared = Arc::new(Shared {
            state: Mutex::new(State { save_to: save_to.to_string(), ..State::default() }),
            changed: Condvar::new(),
            session: format!("{:x}-{:x}", std::process::id(), since.as_nanos()),
            port: listener.local_addr()?.port(),
            editor,
        });
        let serving = shared.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let shared = serving.clone();
                std::thread::spawn(move || {
                    let _ = answer(stream, &shared);
                });
            }
        });
        Ok(Preview { shared })
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/", self.shared.port)
    }

    /// Shows a model's diagrams, or what keeps it from being drawn, on every page open.
    pub fn show(&self, shown: Shown) {
        let mut state = self.shared.state.lock().unwrap();
        state.shown = Some(shown);
        state.closed = false;
        state.version += 1;
        self.shared.changed.notify_all();
    }

    /// Tells every page open that the editor has no model file open now, so it closes.
    pub fn close(&self) {
        let mut state = self.shared.state.lock().unwrap();
        state.closed = true;
        state.version += 1;
        self.shared.changed.notify_all();
    }

    /// Turns the page to a view, in the browser tab already open if there is one,
    /// and otherwise in a new one.
    pub fn open_at(&self, file: &str) -> io::Result<()> {
        let mut state = self.shared.state.lock().unwrap();
        state.version += 1;
        state.select = Some((file.to_string(), state.version));
        self.shared.changed.notify_all();
        if state.watched() {
            return Ok(());
        }
        drop(state);
        self.shared.editor.open_url(&format!("{}#{file}", self.url()))
    }

    /// Opens the page in the browser, unless one left open finds the server first.
    pub fn open_unless_watched(&self) {
        let preview = self.clone();
        std::thread::spawn(move || {
            // A page left open asks again every second.
            std::thread::sleep(Duration::from_millis(1500));
            if !preview.shared.state.lock().unwrap().watched() {
                let _ = preview.shared.editor.open_url(&preview.url());
            }
        });
    }
}

/// FNV-1a, which gives the same number for the same path in every run.
fn hash(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5, |hash, byte| (hash ^ u32::from(*byte)).wrapping_mul(0x0100_0193))
}

/// Answers one request, then closes the connection.
fn answer(stream: TcpStream, shared: &Shared) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut reader = BufReader::new(stream.try_clone()?).take(1 << 16);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let (method, target) = (parts.next().unwrap_or("").to_string(), parts.next().unwrap_or("").to_string());
    let (mut host, mut origin, mut length) = (String::new(), None, 0);
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 || line.trim().is_empty() {
            break;
        }
        let Some((name, value)) = line.split_once(':') else { continue };
        match name.to_ascii_lowercase().as_str() {
            "host" => host = value.trim().to_string(),
            "origin" => origin = Some(value.trim().to_string()),
            "content-length" => length = value.trim().parse().unwrap_or(0),
            _ => {}
        }
    }
    let mut body = vec![0; length.min(1 << 16)];
    reader.read_exact(&mut body)?;
    // The page is for this machine's browser, at this address, alone: a request under
    // another name is another site reaching for it, and so is a post from another page.
    let here = [format!("127.0.0.1:{}", shared.port), format!("localhost:{}", shared.port)];
    let ours = here.contains(&host);
    let from_page = origin.is_some_and(|origin| here.iter().any(|here| origin == format!("http://{here}")));
    let (path, query) = target.split_once('?').unwrap_or((&target, ""));
    let json = "application/json";
    let (status, kind, content) = match (method.as_str(), path) {
        _ if !ours => ("403 Forbidden", "text/plain", "Not the preview's address.".to_string()),
        ("GET", "/") => ("200 OK", "text/html; charset=utf-8", PAGE.to_string()),
        ("GET", "/markdown.js") => ("200 OK", "text/javascript; charset=utf-8", MARKDOWN.to_string()),
        ("GET", "/state") => match state(shared, query, &stream) {
            Some(state) => ("200 OK", json, state.to_string()),
            // The page has gone, and nothing is there to answer.
            None => return Ok(()),
        },
        ("POST", "/open" | "/save") if !from_page => ("403 Forbidden", "text/plain", "Not from the preview.".into()),
        ("POST", "/open") => {
            let asked: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
            ("200 OK", json, json!({ "ok": open(shared, &asked) }).to_string())
        }
        ("POST", "/save") => ("200 OK", json, save(shared).to_string()),
        _ => ("404 Not Found", "text/plain", "Not found.".to_string()),
    };
    let mut stream = stream;
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\n\
         X-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n{content}",
        content.len()
    )?;
    stream.flush()
}

/// What is shown, once it is newer than what the page asking has, or after a while;
/// nothing if the page closes meanwhile.
fn state(shared: &Shared, query: &str, page: &TcpStream) -> Option<Value> {
    let asked = |name: &str| query.split('&').find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='));
    let has = (asked("session") == Some(shared.session.as_str())).then(|| asked("after")?.parse::<u64>().ok());
    let mut state = shared.state.lock().unwrap();
    state.waiting += 1;
    let until = Instant::now() + HOLD;
    let mut gone = false;
    while has.flatten() == Some(state.version) {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        state = shared.changed.wait_timeout(state, left.min(CHECK)).unwrap().0;
        // A page closed, by its user or by itself, stops counting as open at once, so
        // the next model file opened opens it again.
        if hung_up(page) {
            gone = true;
            break;
        }
    }
    state.waiting -= 1;
    if gone {
        return None;
    }
    state.asked = Some(Instant::now());
    let mut answer = json!({
        "session": shared.session, "version": state.version, "saveTo": state.save_to,
        "closed": state.closed, "model": "", "folder": "", "views": [], "errors": [], "notes": [],
    });
    if let Some((file, time)) = &state.select {
        answer["select"] = json!({ "file": file, "time": format!("{}-{time}", shared.session) });
    }
    if let Some(shown) = &state.shown {
        let name = shown.folder.file_name().unwrap_or(shown.folder.as_os_str());
        answer["model"] = json!(name.to_string_lossy());
        answer["folder"] = json!(shown.folder);
        answer["views"] = shown
            .views
            .iter()
            .map(|[file, heading, summary, content]| {
                let markdown = file.ends_with(".md");
                json!({
                    "file": file, "heading": heading, "summary": summary,
                    "svg": if markdown { "" } else { content.as_str() },
                    "markdown": if markdown { content.as_str() } else { "" },
                })
            })
            .collect();
        answer["requirements"] = shown.requirements.clone();
        answer["errors"] = shown.errors.iter().map(Note::json).collect();
        answer["notes"] = shown.notes.iter().map(Note::json).collect();
    }
    Some(answer)
}

/// Whether the page waiting on a connection has gone, closed or with its browser.
fn hung_up(page: &TcpStream) -> bool {
    if page.set_nonblocking(true).is_err() {
        return false;
    }
    let peeked = page.peek(&mut [0]);
    let _ = page.set_nonblocking(false);
    match peeked {
        Ok(read) => read == 0,
        Err(error) => error.kind() != io::ErrorKind::WouldBlock,
    }
}

/// Opens a line of a file of the model shown in the editor. A box of a diagram names
/// its file from the working folder; a link in the table of requirements,
/// `../physical.sysml#L55`, from the folder the page saves the views to. Nothing but
/// the shown model's files is opened.
fn open(shared: &Shared, asked: &Value) -> bool {
    let (folder, save_to) = {
        let state = shared.state.lock().unwrap();
        (state.shown.as_ref().map(|shown| shown.folder.clone()), state.save_to.clone())
    };
    let Some(folder) = folder else { return false };
    let (file, line) = match asked["link"].as_str() {
        Some(link) => {
            let (target, line) = link.split_once("#L").unwrap_or((link, "1"));
            (lexical(&folder.join(&save_to).join(percent_decoded(target))), line.parse().unwrap_or(1))
        }
        None => {
            let (Some(file), Some(line)) = (asked["file"].as_str(), asked["line"].as_u64()) else { return false };
            let Ok(file) = std::path::absolute(file) else { return false };
            (file, line)
        }
    };
    let file = file.canonicalize().unwrap_or(file);
    let Some(parent) = file.parent() else { return false };
    let same = |a: &Path, b: &Path| a == b || a.canonicalize().is_ok_and(|a| b.canonicalize().is_ok_and(|b| a == b));
    if file.extension().is_none_or(|ext| ext != "sysml") || !same(parent, &folder) || !file.is_file() {
        return false;
    }
    let at = json!({ "line": line.saturating_sub(1), "character": 0 });
    let params = json!({ "uri": path_to_uri(&file), "takeFocus": true, "selection": { "start": at, "end": at } });
    shared.editor.request("window/showDocument", params).is_ok()
}

/// A path with each `..` taking the part before it away, as written: the folder the
/// page saves to need not be there yet.
fn lexical(path: &Path) -> PathBuf {
    let mut parts = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::CurDir => {}
            other => parts.push(other),
        }
    }
    parts
}

/// A path as a link writes it, with `%20` for a space and so on, as written.
fn percent_decoded(link: &str) -> String {
    let bytes = link.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |at: usize| bytes.get(at).and_then(|byte| (*byte as char).to_digit(16));
        match (bytes[i], hex(i + 1), hex(i + 2)) {
            (b'%', Some(high), Some(low)) => {
                decoded.push((high * 16 + low) as u8);
                i += 3;
            }
            (byte, _, _) => {
                decoded.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// Writes the views shown into the model's folder for them, as `ortho render` does.
fn save(shared: &Shared) -> Value {
    let state = shared.state.lock().unwrap();
    let Some(shown) = state.shown.as_ref().filter(|shown| !shown.views.is_empty()) else {
        return json!({ "ok": false, "message": "The model does not load now: fix its problems to save its views." });
    };
    let out = shown.folder.join(&state.save_to);
    let written = std::fs::create_dir_all(&out).and_then(|()| {
        shown.views.iter().try_for_each(|[file, _, _, svg]| {
            // A file named after a view is a plain name, never a path out of the folder.
            std::fs::write(out.join(Path::new(file).file_name().unwrap_or_default()), svg)
        })
    });
    match written {
        Ok(()) => {
            let count = shown.views.len();
            let views = if count == 1 { "view" } else { "views" };
            let folder = relative_to_working_folder(&out.to_string_lossy());
            json!({ "ok": true, "message": format!("Wrote {count} {views} to {folder}/") })
        }
        Err(error) => json!({ "ok": false, "message": format!("{}: {error}", out.display()) }),
    }
}
