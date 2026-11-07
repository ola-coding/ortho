//! Writes the VS Code extension's SysML v2 grammar, `editors/vscode/syntaxes/sysml.tmLanguage.json`,
//! from ortho's own keyword list, so the editor colours exactly the words ortho reserves.
//! CI runs it and fails if the committed grammar differs.
//!
//! The grammar follows the lexer (`src/parser/lexer.rs`): `//` starts a note to the end
//! of the line, `//* */` is a note over lines, `/* */` is a comment, `"..."` a string and
//! `'...'` a name, each on one line.
use crate::{Result, write};
use ortho::parser::KEYWORDS;
use serde_json::{Value, json};

const OUT: &str = "editors/vscode/syntaxes/sysml.tmLanguage.json";

/// Words coloured apart from the other keywords, each group with its TextMate scope.
/// Every word here must be one of ortho's keywords; the rest are `keyword.other`.
const GROUPS: &[(&str, &[&str])] = &[
    ("constant.language.sysml", &["true", "false", "null"]),
    ("keyword.operator.word.sysml", &["and", "or", "xor", "not", "implies", "hastype", "istype", "as", "meta", "all"]),
    (
        "storage.modifier.sysml",
        &[
            "private",
            "protected",
            "public",
            "abstract",
            "variation",
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
    keywords.push(json!({ "name": "keyword.other.sysml", "match": words(&others) }));

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
                { "name": "comment.block.sysml", "begin": r"/\*", "end": r"\*/" }
            ]},
            "declarations": { "patterns": [
                {
                    "comment": "The name a definition or package declares: part def Motor, package Power.",
                    "match": format!(r"\b(def|package)\s+(?:(<)\s*{name}\s*(>)\s*)?({name})"),
                    "captures": {
                        "1": { "name": "keyword.other.sysml" },
                        "2": { "name": "punctuation.definition.short-name.sysml" },
                        "3": { "name": "punctuation.definition.short-name.sysml" },
                        "4": { "name": "entity.name.type.sysml" }
                    }
                }
            ]},
            "metadata": { "patterns": [
                { "name": "entity.name.tag.sysml", "match": format!(r"[#@]{name}") }
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
                    "comment": "A name in single quotes, which may hold spaces: part 'flight controller'.",
                    "name": "variable.other.quoted.sysml",
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
    Ok(())
}
