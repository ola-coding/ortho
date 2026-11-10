//! Writes the VS Code extension's SysML v2 grammar, `editors/vscode/syntaxes/sysml.tmLanguage.json`,
//! from ortho's own keyword list, so the editor colours exactly the words ortho reserves,
//! copies LANGUAGE.md to `editors/vscode/ai/LANGUAGE.md`, which the extension's
//! *Add AI instructions* command hands to agents, and copies each example's model to
//! `editors/vscode/examples/`, which its *Open example* command copies for the user,
//! and copies the licence and the icon, which the Marketplace shows.
//! CI runs it and fails if any committed file differs.
//!
//! The grammar follows the lexer (`src/parser/lexer.rs`): `//` starts a note to the end
//! of the line, `//* */` is a note over lines, `/* */` is a comment, `"..."` a string and
//! `'...'` a name, each on one line. A note is a comment to the editor; a `/* */` comment
//! is part of the model, as a docstring is part of Python, and is scoped as documentation.
use crate::{Result, write};
use ortho::parser::KEYWORDS;
use serde_json::{Value, json};

const OUT: &str = "editors/vscode/syntaxes/sysml.tmLanguage.json";
const LANGUAGE_OUT: &str = "editors/vscode/ai/LANGUAGE.md";
const EXAMPLES_OUT: &str = "editors/vscode/examples";
/// Files the extension carries as they are: `vsce` packs only what is in its folder.
const COPIES: [(&str, &str); 2] =
    [("LICENSE", "editors/vscode/LICENSE"), ("logo/ortho-icon.png", "editors/vscode/icon.png")];
/// Where a link to another file of the repository points, once the page is copied out of it.
const REPOSITORY: &str = "https://github.com/ola-coding/ortho/blob/main/";

/// The keywords by what they do, each group with its TextMate scope, as the Python
/// grammar VS Code ships sorts Python's: a declaration's kind is `storage.type`, like
/// `def` and `class`; what directs a behaviour or brings names in is `keyword.control`,
/// like `if` and `import`; words that relate one element to another read as operators.
/// The theme picks the colours; the groups decide which words share one. Every word here
/// must be one of ortho's keywords; the rest are `storage.type`, the kinds of element.
pub const GROUPS: &[(&str, &[&str])] = &[
    ("constant.language.sysml", &["true", "false", "null"]),
    ("keyword.operator.logical.sysml", &["and", "or", "xor", "not", "implies"]),
    (
        "keyword.operator.word.sysml",
        &[
            "specializes",
            "subsets",
            "redefines",
            "references",
            "typed",
            "defined",
            "by",
            "of",
            "to",
            "from",
            "about",
            "via",
            "crosses",
            "hastype",
            "istype",
            "as",
            "meta",
            "all",
            "new",
        ],
    ),
    (
        "storage.modifier.sysml",
        &[
            "private",
            "protected",
            "public",
            "abstract",
            "variation",
            "variant",
            "individual",
            "ordered",
            "nonunique",
            "derived",
            "constant",
            "in",
            "out",
            "inout",
            "end",
            "ref",
            "default",
            "standard",
            "library",
            "parallel",
        ],
    ),
    (
        "keyword.control.sysml",
        &[
            "import",
            "expose",
            "filter",
            "render",
            "accept",
            "send",
            "assign",
            "do",
            "entry",
            "exit",
            "if",
            "else",
            "then",
            "first",
            "loop",
            "while",
            "until",
            "for",
            "fork",
            "join",
            "merge",
            "decide",
            "terminate",
            "return",
            "after",
            "at",
            "when",
        ],
    ),
    (
        "keyword.other.sysml",
        &[
            "perform", "exhibit", "include", "satisfy", "verify", "assert", "assume", "require", "allocate", "bind",
            "connect", "frame", "language", "locale",
        ],
    ),
];

/// An alternation of whole words, in the given order.
fn words(list: &[&str]) -> String {
    format!(r"\b(?:{})\b", list.join("|"))
}

fn grammar() -> Result<Value> {
    for (_, group) in GROUPS {
        if let Some(stray) = group.iter().find(|word| !KEYWORDS.contains(word)) {
            return Err(format!("'{stray}' is not one of ortho's keywords"));
        }
    }
    let grouped = |word: &&str| GROUPS.iter().any(|(_, group)| group.contains(word));
    let others: Vec<&str> = KEYWORDS.iter().copied().filter(|word| !grouped(word)).collect();
    let name = r"(?:[A-Za-z_][A-Za-z0-9_]*|'(?:[^'\\\r\n]|\\.)*')";

    let mut keywords: Vec<Value> =
        GROUPS.iter().map(|(scope, group)| json!({ "name": scope, "match": words(group) })).collect();
    keywords.push(json!({ "name": "storage.type.sysml", "match": words(&others) }));

    Ok(json!({
        "$schema": "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
        "name": "SysML v2",
        "scopeName": "source.sysml",
        "comment": "Written by `cargo xtask vscode-grammar` from ortho's keyword list. Do not edit by hand.",
        "patterns": [
            { "include": "#comments" },
            { "include": "#declarations" },
            { "include": "#metadata" },
            { "include": "#keywords" },
            { "include": "#strings" },
            { "include": "#names" },
            { "include": "#numbers" },
            { "include": "#operators" }
        ],
        "repository": {
            "comments": { "patterns": [
                { "name": "comment.block.note.sysml", "begin": r"//\*", "end": r"\*/" },
                { "name": "comment.line.double-slash.sysml", "match": "//.*$" },
                { "name": "comment.block.documentation.sysml", "begin": r"/\*", "end": r"\*/" }
            ]},
            "declarations": { "patterns": [
                {
                    "comment": "The name a definition declares, as a class's: part def Motor.",
                    "match": format!(r"\b(def)\s+(?:(<)\s*{name}\s*(>)\s*)?({name})"),
                    "captures": {
                        "1": { "name": "storage.type.sysml" },
                        "2": { "name": "punctuation.definition.short-name.sysml" },
                        "3": { "name": "punctuation.definition.short-name.sysml" },
                        "4": { "name": "entity.name.type.sysml" }
                    }
                },
                {
                    "comment": "The name a package declares: package Power.",
                    "match": format!(r"\b(package)\s+(?:(<)\s*{name}\s*(>)\s*)?({name})"),
                    "captures": {
                        "1": { "name": "storage.type.sysml" },
                        "2": { "name": "punctuation.definition.short-name.sysml" },
                        "3": { "name": "punctuation.definition.short-name.sysml" },
                        "4": { "name": "entity.name.namespace.sysml" }
                    }
                }
            ]},
            "metadata": { "patterns": [
                {
                    "comment": "Metadata on an element, as a decorator on a Python function: #refinement.",
                    "match": format!(r"([#@])({name})"),
                    "captures": {
                        "1": { "name": "punctuation.definition.decorator.sysml" },
                        "2": { "name": "entity.name.function.decorator.sysml" }
                    }
                }
            ]},
            "keywords": { "patterns": keywords },
            "strings": { "patterns": [
                {
                    "name": "string.quoted.double.sysml",
                    "begin": "\"",
                    "end": r#""|$"#,
                    "patterns": [{ "name": "constant.character.escape.sysml", "match": r"\\." }]
                }
            ]},
            "names": { "patterns": [
                {
                    "comment": "A name in single quotes, which may hold spaces: part 'flight controller'. A name, not a string: it keeps the text's colour.",
                    "name": "meta.name.quoted.sysml",
                    "match": r"'(?:[^'\\\r\n]|\\.)*'"
                }
            ]},
            "numbers": { "patterns": [
                { "name": "constant.numeric.sysml", "match": r"\b[0-9]+(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?\b|\.[0-9]+(?:[eE][+-]?[0-9]+)?\b" }
            ]},
            "operators": { "patterns": [
                { "name": "keyword.operator.sysml", "match": r":>>|::>|:>|::|->|=>|:=|\.\.|===|!==|==|!=|<=|>=|\?\?|~" }
            ]}
        }
    }))
}

pub fn run() -> Result {
    let text = serde_json::to_string_pretty(&grammar()?).map_err(|error| error.to_string())?;
    write(OUT, &format!("{text}\n"))?;
    println!("Wrote {OUT}");
    let language = std::fs::read_to_string("LANGUAGE.md").map_err(|error| format!("LANGUAGE.md: {error}"))?;
    std::fs::create_dir_all("editors/vscode/ai").map_err(|error| error.to_string())?;
    write(LANGUAGE_OUT, &language_copy(&language, "vscode-grammar"))?;
    println!("Wrote {LANGUAGE_OUT}");
    copy_examples(EXAMPLES_OUT)?;
    println!("Wrote {EXAMPLES_OUT}");
    for (from, to) in COPIES {
        std::fs::copy(from, to).map_err(|error| format!("{from}: {error}"))?;
        println!("Wrote {to}");
    }
    Ok(())
}

/// Each example's `.sysml` files, as they are, without the diagrams: the preview draws
/// those. The folder is written afresh, so a file removed from an example goes too.
pub fn copy_examples(out_folder: &str) -> Result {
    let error = |error: std::io::Error| error.to_string();
    if std::path::Path::new(out_folder).exists() {
        std::fs::remove_dir_all(out_folder).map_err(error)?;
    }
    let mut examples: Vec<_> =
        std::fs::read_dir("examples").map_err(error)?.collect::<std::io::Result<_>>().map_err(error)?;
    examples.sort_by_key(|entry| entry.file_name());
    for example in examples.iter().filter(|entry| entry.path().is_dir()) {
        let out = std::path::Path::new(out_folder).join(example.file_name());
        std::fs::create_dir_all(&out).map_err(error)?;
        for file in std::fs::read_dir(example.path()).map_err(error)? {
            let path = file.map_err(error)?.path();
            if path.extension().is_some_and(|extension| extension == "sysml") {
                std::fs::copy(&path, out.join(path.file_name().expect("a file has a name"))).map_err(error)?;
            }
        }
    }
    Ok(())
}

/// LANGUAGE.md as it reads outside the repository: a link to another of its files,
/// `[VIEWS.md](VIEWS.md)`, points at it on GitHub. A link within the page stays.
pub fn language_copy(language: &str, task: &str) -> String {
    let mut copy = format!("<!-- Copied from ortho's LANGUAGE.md by `cargo xtask {task}`. -->\n\n");
    let mut rest = language;
    while let Some(at) = rest.find("](") {
        let (before, after) = rest.split_at(at + 2);
        copy.push_str(before);
        let relative = !after.starts_with("http") && !after.starts_with('#');
        if relative {
            copy.push_str(REPOSITORY);
        }
        rest = after;
    }
    copy.push_str(rest);
    copy
}
