//! Writes the Zed extension's highlighting, `editors/zed/languages/sysml/highlights.scm`,
//! from the keyword groups of the VS Code grammar, so both editors colour the same words
//! alike, and copies the licence into the extension, which Zed's extension registry
//! requires. CI runs it and fails if any committed file differs.
//!
//! Zed parses with Nomograph's Tree-sitter grammar, whose keywords are its anonymous
//! nodes. A query naming a node the grammar lacks fails as a whole, so the words that
//! grammar does not have are left out; `null` it reads as a literal.
use crate::vscode_grammar::GROUPS;
use crate::{Result, write};
use ortho::parser::KEYWORDS;

const OUT: &str = "editors/zed/languages/sysml/highlights.scm";
const COPIES: [(&str, &str); 1] = [("LICENSE", "editors/zed/LICENSE")];

/// ortho's keywords that are not keywords of the Tree-sitter grammar: `typed by` it
/// reads as `:`, and the literals it reads as literals, which are coloured as such.
const NOT_KEYWORDS_THERE: [&str; 4] = ["typed", "null", "true", "false"];

/// The grammar's definitions, each declaring its name as `part def Motor` does.
const DEFINITIONS: [&str; 26] = [
    "action_definition",
    "allocation_definition",
    "analysis_definition",
    "attribute_definition",
    "calc_definition",
    "case_definition",
    "concern_definition",
    "connection_definition",
    "constraint_definition",
    "enumeration_definition",
    "flow_definition",
    "generic_definition",
    "individual_definition",
    "interface_definition",
    "item_definition",
    "metadata_definition",
    "occurrence_definition",
    "part_definition",
    "port_definition",
    "rendering_definition",
    "requirement_definition",
    "state_definition",
    "use_case_definition",
    "verification_definition",
    "view_definition",
    "viewpoint_definition",
];

/// Each TextMate scope as the Zed capture that themes colour alike, as Zed's Python
/// highlighting captures Python's words. A capture a theme lacks falls back to the
/// part before its last dot.
fn capture(scope: &str) -> &'static str {
    match scope {
        "keyword.operator.logical.sysml" | "keyword.operator.word.sysml" => "keyword.operator",
        "storage.modifier.sysml" => "keyword.modifier",
        "keyword.control.sysml" => "keyword.control",
        _ => "keyword",
    }
}

fn kept(word: &&str) -> bool {
    !NOT_KEYWORDS_THERE.contains(word)
}

fn list(words: &[&str]) -> String {
    let mut lines = String::from("[\n");
    for word in words {
        lines.push_str(&format!("  \"{word}\"\n"));
    }
    lines.push(']');
    lines
}

fn highlights() -> String {
    let mut out =
        String::from("; Written by `cargo xtask zed-grammar` from ortho's keyword list. Do not edit by hand.\n\n");
    for (scope, group) in GROUPS {
        let words: Vec<&str> = group.iter().copied().filter(kept).collect();
        if !words.is_empty() {
            out.push_str(&format!("; {scope}\n{} @{}\n\n", list(&words), capture(scope)));
        }
    }
    let grouped = |word: &&str| GROUPS.iter().any(|(_, group)| group.contains(word));
    let kinds: Vec<&str> = KEYWORDS.iter().copied().filter(|word| !grouped(word)).filter(kept).collect();
    out.push_str(&format!("; The kinds of element, as `def` and `class` in Python\n{} @keyword\n\n", list(&kinds)));
    out.push_str("; The name a definition declares, as a class's: part def Motor.\n");
    for definition in DEFINITIONS {
        out.push_str(&format!("({definition} (identification (name) @type))\n"));
    }
    out.push('\n');
    out.push_str(TREES);
    out
}

/// What is highlighted by its place in the tree, as the VS Code grammar does by pattern.
const TREES: &str = r##"; The name a package declares: package Power.
(package_definition (identification (name) @type @namespace))
(library_package (identification (name) @type @namespace))

; Metadata on an element, as a decorator on a Python function: #refinement.
(prefix_metadata "#" @punctuation.special (qualified_name) @function)
(metadata_usage "@" @punctuation.special (qualified_name) @function)

(boolean_literal) @boolean
(null_literal) @constant.builtin
(string_literal) @string
(integer_literal) @number
(real_literal) @number

[":>>" "::>" ":>" "::" "->" ":=" ".." "==" "!=" "<=" ">=" "??" "~"] @operator

; A note, // or //* */, is not part of the model; a /* */ comment is, as a docstring
; is part of Python.
((comment) @comment
  (#not-match? @comment "^/\\*"))
((comment) @comment.doc
  (#match? @comment.doc "^/\\*"))
(block_comment_body) @comment.doc
"##;

pub fn run() -> Result {
    write(OUT, &highlights())?;
    println!("Wrote {OUT}");
    for (from, to) in COPIES {
        std::fs::copy(from, to).map_err(|error| format!("{from}: {error}"))?;
        println!("Wrote {to}");
    }
    Ok(())
}
