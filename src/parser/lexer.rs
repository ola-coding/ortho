//! The tokens of SysML v2's textual notation. A keyword is reserved, so `to` is never a
//! name; a symbol is the longest that matches; `/* */` is a token and notes are not.
use super::SyntaxError;

#[rustfmt::skip]
pub const KEYWORDS: &[&str] = &[
    "about", "abstract", "accept", "action", "actor", "after", "alias", "all", "allocate", "allocation", "analysis",
    "and", "as", "assert", "assign", "assume", "at", "attribute", "bind", "binding", "by", "calc", "case", "comment",
    "concern", "connect", "connection", "constant", "constraint", "crosses", "decide", "def", "default", "defined",
    "dependency", "derived", "do", "doc", "else", "end", "entry", "enum", "event", "exhibit", "exit", "expose",
    "false", "filter", "first", "flow", "for", "fork", "frame", "from", "hastype", "if", "implies", "import", "in",
    "include", "individual", "inout", "interface", "istype", "item", "join", "language", "library", "locale", "loop",
    "merge", "message", "meta", "metadata", "new", "nonunique", "not", "null", "objective", "occurrence", "of", "or",
    "ordered", "out", "package", "parallel", "part", "perform", "port", "private", "protected", "public", "redefines",
    "ref", "references", "render", "rendering", "rep", "require", "requirement", "return", "satisfy", "send",
    "snapshot", "specializes", "stakeholder", "standard", "state", "subject", "subsets", "succession", "terminate",
    "then", "timeslice", "to", "transition", "true", "typed", "until", "use", "variant", "variation", "verification",
    "verify", "via", "view", "viewpoint", "when", "while", "xor",
];

// Longest first.
#[rustfmt::skip]
const SYMBOLS: &[&str] = &[
    "::**", "!==", "::*", "::>", ":>>", "===", "!=", "**", "->", "..", ".?", "::", ":=", ":>", "<=", "==", "=>", ">=",
    "??", "@@", "#", "%", "&", "(", ")", "*", "+", ",", "-", ".", "/", ":", ";", "<", "=", ">", "?", "@", "[", "]",
    "^", "{", "|", "}", "~",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Word,
    Id,
    Number,
    Str,
    Unrestricted,
    Comment,
    Eof,
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub kind: Kind,
    pub text: &'a str,
    pub start: usize,
}

fn is_word_start(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphabetic()
}

fn is_word(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphanumeric()
}

/// What JavaScript's `\s` matches.
fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

/// The end of a `"..."` or `'...'` starting at `start`, on one line.
fn quoted_end(text: &str, start: usize, quote: u8) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b if b == quote => return Some(i + 1),
            b'\\' => {
                let c = text[i + 1..].chars().next()?;
                if matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                    return None;
                }
                i += 1 + c.len_utf8();
            }
            b'\n' | b'\r' => return None,
            _ => i += 1,
        }
    }
    None
}

fn number_end(bytes: &[u8], start: usize) -> usize {
    let digits = |mut i: usize| {
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        i
    };
    let mut end = digits(start);
    if end + 1 < bytes.len() && bytes[end] == b'.' && bytes[end + 1].is_ascii_digit() {
        end = digits(end + 1);
    }
    if end < bytes.len() && (bytes[end] == b'e' || bytes[end] == b'E') {
        let mut i = end + 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        if i < bytes.len() && bytes[i].is_ascii_digit() {
            end = digits(i);
        }
    }
    end
}

fn unexpected(text: &str, offset: usize) -> SyntaxError {
    let character = text[offset..].chars().next().unwrap_or(' ');
    SyntaxError { offset, message: format!("Unexpected character '{character}'.") }
}

/// Every token of `text`, ending in `Eof`, or what no token matches.
pub fn tokenize(text: &str) -> Result<Vec<Token<'_>>, SyntaxError> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::with_capacity(text.len() / 5);
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b >= 0x80 {
            let c = text[i..].chars().next().unwrap();
            if is_space(c) {
                i += c.len_utf8();
                continue;
            }
            return Err(unexpected(text, i));
        }
        if is_space(b as char) {
            i += 1;
            continue;
        }
        let (kind, end) = if b == b'/' && bytes.get(i + 1) == Some(&b'/') {
            let line_end = text[i..].find(['\n', '\r']).map_or(bytes.len(), |n| i + n);
            let note_end =
                if bytes.get(i + 2) == Some(&b'*') { text[i + 3..].find("*/").map(|n| i + 3 + n + 2) } else { None };
            i = note_end.unwrap_or(line_end);
            continue;
        } else if b == b'/' && bytes.get(i + 1) == Some(&b'*') && text[i + 2..].contains("*/") {
            (Kind::Comment, i + 2 + text[i + 2..].find("*/").unwrap() + 2)
        } else if is_word_start(b) {
            let mut end = i + 1;
            while end < bytes.len() && is_word(bytes[end]) {
                end += 1;
            }
            (if KEYWORDS.binary_search(&&text[i..end]).is_ok() { Kind::Word } else { Kind::Id }, end)
        } else if b.is_ascii_digit() {
            (Kind::Number, number_end(bytes, i))
        } else if b == b'"' {
            (Kind::Str, quoted_end(text, i, b'"').ok_or_else(|| unexpected(text, i))?)
        } else if b == b'\'' {
            (Kind::Unrestricted, quoted_end(text, i, b'\'').ok_or_else(|| unexpected(text, i))?)
        } else {
            let symbol = SYMBOLS.iter().find(|s| text[i..].starts_with(**s)).ok_or_else(|| unexpected(text, i))?;
            (Kind::Word, i + symbol.len())
        };
        tokens.push(Token { kind, text: &text[i..end], start: i });
        i = end;
    }
    tokens.push(Token { kind: Kind::Eof, text: "", start: bytes.len() });
    Ok(tokens)
}
