//! Builds the pitch deck from `marketing/deck.template.html` by inlining the committed
//! diagrams, syntax-highlighting the SysML excerpts, counting the tests and numbering
//! the sheets.
use crate::{Result, read, write};
use ortho::parser::KEYWORDS;
use regex::{Captures, Regex};
use std::collections::BTreeSet;
use std::process::{Command, Stdio};

const TEMPLATE: &str = "marketing/deck.template.html";
const STANDALONE_OUT: &str = "marketing/ortho-deck.html";
const ARTIFACT_OUT: &str = "marketing/ortho-deck.artifact.html";

/// Short prefixes used in the template's data-svg references.
const DIAGRAM_DIRS: &[(&str, &str)] =
    &[("drone", "examples/survey-drone/diagrams"), ("coffee", "examples/coffee-machine/diagrams")];

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// One pass over the code, so highlighting never matches inside markup it wrote.
fn highlight_sysml(code: &str) -> String {
    let pattern = Regex::new(r#"(//[^\r\n]*)|("[^"]*")|(\b\d+(?:\.\d+)?\b)|([A-Za-z_][A-Za-z0-9_]*)|(::|:>)"#).unwrap();
    let mut out = String::new();
    let mut last = 0;
    for m in pattern.captures_iter(code) {
        let whole = m.get(0).unwrap();
        out += &escape_html(&code[last..whole.start()]);
        last = whole.end();
        let text = whole.as_str();
        out += &if m.get(1).is_some() {
            format!(r#"<span class="t-comment">{}</span>"#, escape_html(text))
        } else if m.get(2).is_some() {
            format!(r#"<span class="t-string">{}</span>"#, escape_html(text))
        } else if m.get(3).is_some() {
            format!(r#"<span class="t-number">{text}</span>"#)
        } else if m.get(5).is_some() {
            format!(r#"<span class="t-op">{}</span>"#, escape_html(text))
        } else if KEYWORDS.contains(&text) {
            format!(r#"<span class="t-keyword">{text}</span>"#)
        } else if text.starts_with(|c: char| c.is_ascii_uppercase()) {
            format!(r#"<span class="t-type">{text}</span>"#)
        } else {
            escape_html(text)
        };
    }
    out + &escape_html(&code[last..])
}

/// "drone/logical-view-survey-drone.svg" becomes the file's path, and a key unique to it.
fn resolve_ref(reference: &str) -> Result<(String, String)> {
    let (prefix, file) = reference.split_once('/').unwrap_or((reference, ""));
    let dir = DIAGRAM_DIRS.iter().find(|(p, _)| *p == prefix).map(|(_, dir)| dir);
    match dir {
        Some(dir) if !file.is_empty() => {
            Ok((format!("{dir}/{file}"), format!("{prefix}-{}", file.replacen(".svg", "", 1))))
        }
        _ => {
            let prefixes: Vec<&str> = DIAGRAM_DIRS.iter().map(|(p, _)| *p).collect();
            Err(format!(
                "Unknown diagram reference \"{reference}\" (expected one of {}/<file>.svg)",
                prefixes.join(", ")
            ))
        }
    }
}

/// Inlines an SVG: gives its ids the key as a prefix, since every diagram declares the
/// same marker ids and ids in one document must differ, and drops the fixed width and
/// height so CSS can scale it against the viewBox.
fn prepare_svg(svg: &str, key: &str) -> String {
    let body = Regex::new(r"<\?xml[^>]*\?>").unwrap().replace_all(svg, "");
    let body = Regex::new(r#"(<svg[^>]*?)\swidth="[^"]*""#).unwrap().replace(&body, "${1}");
    let mut body = Regex::new(r#"(<svg[^>]*?)\sheight="[^"]*""#).unwrap().replace(&body, "${1}").into_owned();
    let ids: Vec<String> =
        Regex::new(r#"\sid="([^"]+)""#).unwrap().captures_iter(&body).map(|c| c[1].to_string()).collect();
    for id in ids {
        let quoted = regex::escape(&id);
        body = Regex::new(&format!(r#"\sid="{quoted}""#))
            .unwrap()
            .replace_all(&body, regex::NoExpand(&format!(r#" id="{key}-{id}""#)))
            .into_owned();
        body = Regex::new(&format!(r"url\(#{quoted}\)"))
            .unwrap()
            .replace_all(&body, regex::NoExpand(&format!("url(#{key}-{id})")))
            .into_owned();
    }
    body
}

/// How many tests the suite holds, as `cargo test -- --list` counts them, so the deck
/// never quotes a stale number.
fn count_tests() -> Result<usize> {
    let output = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()))
        .args(["test", "--quiet", "--package", "ortho", "--", "--list"])
        .stderr(Stdio::inherit())
        .output()
        .map_err(|error| format!("cargo test: {error}"))?;
    if !output.status.success() {
        return Err("cargo test -- --list failed".to_string());
    }
    let listing = String::from_utf8_lossy(&output.stdout);
    let count = listing.lines().filter(|line| line.ends_with(": test")).count();
    if count == 0 {
        return Err("cargo test -- --list found no tests; refusing to quote a count.".to_string());
    }
    Ok(count)
}

pub fn run() -> Result {
    let mut html = read(TEMPLATE)?;

    // Inline every diagram the template asks for, so adding a figure needs no change here.
    let references: BTreeSet<String> =
        Regex::new(r#"data-svg="([^"]+)""#).unwrap().captures_iter(&html).map(|c| c[1].to_string()).collect();
    for reference in references {
        let (path, key) = resolve_ref(&reference)?;
        let figure = format!(r#"<div class="figure-svg">{}</div>"#, prepare_svg(&read(&path)?, &key));
        html = html.replacen(&format!(r#"<div class="figure-svg" data-svg="{reference}"></div>"#), &figure, 1);
    }

    // The logo, inlined like the diagrams so the published deck is one file. Its ink
    // and teal become the deck's logo colours, so it follows the theme.
    let logo = prepare_svg(&read("logo/ortho-logo.svg")?, "logo")
        .replace(r##"stroke="#2b2b2b""##, r#"style="stroke: var(--logo-ink)""#)
        .replace(r##"fill="#2a8fa3""##, r#"style="fill: var(--logo-teal)""#);
    html = html.replace(r#"<div class="logo"></div>"#, &format!(r#"<div class="logo">{logo}</div>"#));

    html = html.replace("{{TESTS}}", &count_tests()?.to_string());

    html = Regex::new(r#"<pre class="code"([^>]*)>([\s\S]*?)</pre>"#)
        .unwrap()
        .replace_all(&html, |c: &Captures| {
            let code = c[2].strip_prefix('\n').unwrap_or(&c[2]).trim_end();
            format!(r#"<pre class="code"{}>{}</pre>"#, &c[1], highlight_sysml(code))
        })
        .into_owned();

    // Number the sheets from their order in the document.
    let total = html.matches(r#"<section class="sheet"#).count();
    let slot = r#"<span class="sheet-no"></span>"#;
    let mut sheet = 0;
    while html.contains(slot) {
        sheet += 1;
        html = html.replacen(slot, &format!(r#"<span class="sheet-no">Sheet {sheet:02} / {total:02}</span>"#), 1);
    }
    if sheet != total {
        return Err(format!("Found {total} sheets but {sheet} sheet-number slots; every sheet needs one."));
    }

    let unresolved: BTreeSet<&str> =
        Regex::new(r#"\{\{[^}]+\}\}|data-svg=""#).unwrap().find_iter(&html).map(|m| m.as_str()).collect();
    if !unresolved.is_empty() {
        return Err(format!(
            "The deck template has unresolved placeholders: {}",
            unresolved.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    write(ARTIFACT_OUT, &html)?;
    write(
        STANDALONE_OUT,
        &format!(
            "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\" />\n\
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n\
             <title>ortho — orthographic projection for architecture</title>\n</head>\n<body>\n{html}\n</body>\n</html>\n"
        ),
    )?;
    println!("Wrote {STANDALONE_OUT} and {ARTIFACT_OUT} ({total} sheets)");
    Ok(())
}
