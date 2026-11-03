//! Reads SysML v2 text into a tree.
pub mod ast;
mod grammar;
mod lexer;
pub use lexer::KEYWORDS;
mod scope;
mod validator;
pub mod workspace;

use ast::{Ast, NodeId};

/// Why a file does not parse. A file stops at its first syntax error.
#[derive(Debug)]
pub struct SyntaxError {
    /// Where, in bytes.
    pub offset: usize,
    pub message: String,
}

/// Adds a file's tree to `ast` and returns its root. A file that does not parse adds nothing.
pub fn parse(ast: &mut Ast, text: &str) -> Result<NodeId, SyntaxError> {
    grammar::parse(ast, text)
}

/// The line and character of `offset`, from 1, as an editor counts them: a character
/// outside the basic plane counts as two.
pub fn position(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset];
    let line_start = before.rfind(['\n', '\r']).map_or(0, |i| i + 1);
    let line = before.match_indices(['\n', '\r']).filter(|(i, _)| !before[*i..].starts_with("\r\n")).count();
    (line + 1, before[line_start..].encode_utf16().count() + 1)
}
