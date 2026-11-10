//! The Language Server Protocol's framing and its file names: messages read and written
//! as `Content-Length` headers and JSON, and `file:` URIs turned into paths and back.
use serde_json::Value;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

/// The next message, or nothing once the editor closes the stream.
pub fn read_message(input: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut length: Option<usize> = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            // A blank line before any header is noise between messages.
            if length.is_some() {
                break;
            }
            continue;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().ok();
        }
    }
    let mut body = vec![0; length.unwrap_or(0)];
    input.read_exact(&mut body)?;
    serde_json::from_slice(&body).map(Some).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

pub fn write_message(output: &mut impl Write, message: &Value) -> io::Result<()> {
    let body = message.to_string();
    write!(output, "Content-Length: {}\r\n\r\n{body}", body.len())?;
    output.flush()
}

/// The path a `file:` URI names, or nothing for another kind of URI.
pub fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    // `file:///path`, or `file://localhost/path`; a share on another host is
    // `//host/path` on Windows.
    let at = rest.find('/')?;
    let (host, path) = (&rest[..at], &rest[at..]);
    let path = percent_decode(path)?;
    let path = if host.is_empty() || host.eq_ignore_ascii_case("localhost") { path } else { format!("//{host}{path}") };
    // `/C:/Users` is `C:/Users`.
    let bytes = path.as_bytes();
    let drive = bytes.len() >= 3 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b':';
    Some(PathBuf::from(if drive { &path[1..] } else { &path[..] }))
}

/// A `file:` URI for an absolute path.
pub fn path_to_uri(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    let path = if path.starts_with('/') { path } else { format!("/{path}") };
    let mut uri = String::from("file://");
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~:".contains(&byte) {
            uri.push(byte as char);
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

/// The byte offset of an editor's position, a line and a character from 0, where a
/// character outside the basic plane counts as two. A position past the end of its
/// line is the end of the line.
pub fn offset_at(text: &str, line: usize, character: usize) -> usize {
    let mut start = 0;
    for _ in 0..line {
        match text[start..].find(['\n', '\r']) {
            Some(at) => {
                start += at;
                start += if text[start..].starts_with("\r\n") { 2 } else { 1 };
            }
            None => return text.len(),
        }
    }
    let mut units = 0;
    for (at, char) in text[start..].char_indices() {
        if units >= character || char == '\n' || char == '\r' {
            return start + at;
        }
        units += char.len_utf16();
    }
    text.len()
}

/// An editor's position of a byte offset: a line and a character from 0.
pub fn position_of(text: &str, offset: usize) -> Value {
    let (line, character) = crate::parser::position(text, offset.min(text.len()));
    serde_json::json!({ "line": line - 1, "character": character - 1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uris_and_paths() {
        assert_eq!(uri_to_path("file:///home/a%20b/m.sysml"), Some(PathBuf::from("/home/a b/m.sysml")));
        assert_eq!(uri_to_path("file:///c%3A/Users/m.sysml"), Some(PathBuf::from("c:/Users/m.sysml")));
        assert_eq!(uri_to_path("untitled:Untitled-1"), None);
        assert_eq!(path_to_uri(Path::new("/home/a b/m.sysml")), "file:///home/a%20b/m.sysml");
        assert_eq!(path_to_uri(Path::new(r"C:\Users\m.sysml")), "file:///C:/Users/m.sysml");
    }

    #[test]
    fn positions_and_offsets() {
        let text = "ab\r\nc😀d\n";
        assert_eq!(offset_at(text, 0, 1), 1);
        assert_eq!(offset_at(text, 1, 0), 4);
        assert_eq!(offset_at(text, 1, 3), 9);
        assert_eq!(offset_at(text, 0, 99), 2);
        assert_eq!(offset_at(text, 9, 0), text.len());
        assert_eq!(position_of(text, 9), serde_json::json!({ "line": 1, "character": 3 }));
    }
}
