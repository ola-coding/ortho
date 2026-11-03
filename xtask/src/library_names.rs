//! Lists the names OMG's standard library declares, package by package, into
//! `src/model/library.rs`, so that a view may name a library element without ortho
//! loading the library. Run after `cargo xtask omg-models` has fetched the library.
//!
//! Only names are kept: a declaration is read as far as its name, and every body that
//! is not a package's is skipped.
use crate::{Result, write};
use ortho::parser::KEYWORDS;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const LIBRARY: &str = ".omg-models/sysml.library";
const OUT: &str = "src/model/library.rs";

/// KerML's reserved words that SysML's do not include (KerML §8.2.2.6).
const KERML_WORDS: &[&str] = &[
    "assoc",
    "behavior",
    "bool",
    "chains",
    "class",
    "classifier",
    "composite",
    "conjugate",
    "conjugates",
    "conjugation",
    "connector",
    "const",
    "datatype",
    "differences",
    "disjoining",
    "disjoint",
    "expr",
    "feature",
    "featured",
    "featuring",
    "function",
    "interaction",
    "intersects",
    "inv",
    "inverse",
    "inverting",
    "member",
    "metaclass",
    "multiplicity",
    "namespace",
    "portion",
    "predicate",
    "redefinition",
    "specialization",
    "step",
    "struct",
    "subclassifier",
    "subset",
    "subtype",
    "type",
    "typed",
    "typing",
    "unions",
    "var",
];
const SKIPPED: &[&str] = &["doc", "comment", "rep", "language", "filter", "dependency", "expose", "render"];

#[derive(Default)]
struct Package {
    names: BTreeSet<String>,
    imports: BTreeSet<String>,
}

struct Scanner {
    packages: BTreeMap<String, Package>,
    plain_name: Regex,
}

fn files(dir: &Path, out: &mut Vec<PathBuf>) -> Result {
    let entries =
        std::fs::read_dir(dir).map_err(|error| format!("{}: {error}; run cargo xtask omg-models", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir() {
            files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "sysml" || e == "kerml") {
            out.push(path);
        }
    }
    Ok(())
}

fn tokens_of(text: &str) -> Vec<String> {
    let code = Regex::new(r"//\*[\s\S]*?\*/").unwrap().replace_all(text, " ");
    // A comment ends the `doc` or `comment` element it is the body of.
    let code = Regex::new(r"/\*[\s\S]*?\*/").unwrap().replace_all(&code, " ; ");
    let code = Regex::new(r"//[^\n]*").unwrap().replace_all(&code, " ");
    let code = Regex::new(r#""(?:\\.|[^"\\])*""#).unwrap().replace_all(&code, r#" "" "#);
    Regex::new(r"'(?:\\.|[^'\\])*'|[A-Za-z_][A-Za-z0-9_]*|::\*\*|::\*|::|[^\s0-9A-Za-z_]")
        .unwrap()
        .find_iter(&code)
        .map(|m| m.as_str().to_string())
        .collect()
}

fn unquote(token: &str) -> String {
    match token.strip_prefix('\'').and_then(|t| t.strip_suffix('\'')) {
        Some(inner) => Regex::new(r"\\(.)").unwrap().replace_all(inner, "$1").into_owned(),
        None => token.to_string(),
    }
}

impl Scanner {
    fn is_name(&self, token: Option<&String>) -> bool {
        token.is_some_and(|t| {
            t.starts_with('\'')
                || (self.plain_name.is_match(t)
                    && !KEYWORDS.contains(&t.as_str())
                    && !KERML_WORDS.contains(&t.as_str()))
        })
    }

    fn package(&mut self, name: &str) -> &mut Package {
        self.packages.entry(name.to_string()).or_default()
    }

    /// The names a declaration gives, its short name included: `<kg> kilogram`.
    fn declared(&self, words: &[String]) -> Vec<String> {
        let mut names = Vec::new();
        let mut k = 0;
        while k < words.len() {
            let word = words[k].as_str();
            if word == "#" || word == "@" {
                // A metadata keyword, `#Safety`, or annotation names nothing here.
                k += 1;
                while words.get(k + 1).is_some_and(|w| w == "::") {
                    k += 2;
                }
            } else if word == "<" && self.is_name(words.get(k + 1)) && words.get(k + 2).is_some_and(|w| w == ">") {
                names.push(unquote(&words[k + 1]));
                k += 2;
            } else if self.is_name(Some(&words[k])) {
                names.push(unquote(word));
                break;
            } else if [":", "=", "[", "(", "~", "."].contains(&word) {
                break;
            }
            k += 1;
        }
        names
    }

    fn scan(&mut self, tokens: &[String]) {
        // The package each open brace belongs to, or None for any other body.
        let mut stack: Vec<Option<String>> = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            if tokens[i] == "}" {
                stack.pop();
                i += 1;
                continue;
            }
            let mut j = i;
            while j < tokens.len() && !["{", "}", ";"].contains(&tokens[j].as_str()) {
                j += 1;
            }
            let words = &tokens[i..j];
            let owner: Option<String> = match stack.last() {
                None => Some(String::new()),
                Some(owner) => owner.clone(),
            };
            let mut opened: Option<String> = None;
            if let Some(owner) = owner
                && !words.is_empty()
                && !SKIPPED.contains(&words[0].as_str())
            {
                let has = |word: &str| words.iter().position(|w| w == word);
                if let Some(at) = has("package") {
                    if let Some(name) = self.declared(&words[at + 1..]).into_iter().next() {
                        let full = if owner.is_empty() { name.clone() } else { format!("{owner}::{name}") };
                        self.package(&full);
                        if !owner.is_empty() {
                            self.package(&owner).names.insert(name);
                        }
                        opened = Some(full);
                    }
                } else if !owner.is_empty()
                    && let Some(at) = has("import")
                {
                    let path: Vec<&String> = words[at + 1..].iter().filter(|w| *w != "all").collect();
                    if has("public").is_some()
                        && let Some(last) = path.last()
                    {
                        if *last == "::*" || *last == "::**" {
                            let imported: Vec<String> =
                                path[..path.len() - 1].iter().filter(|w| **w != "::").map(|w| unquote(w)).collect();
                            self.package(&owner).imports.insert(imported.join("::"));
                        } else if self.is_name(Some(last)) {
                            let name = unquote(last);
                            self.package(&owner).names.insert(name);
                        }
                    }
                } else if !owner.is_empty()
                    && let Some(at) = has("alias")
                {
                    for name in self.declared(&words[at + 1..]) {
                        self.package(&owner).names.insert(name);
                    }
                } else if !owner.is_empty() {
                    for name in self.declared(words) {
                        self.package(&owner).names.insert(name);
                    }
                }
            }
            if tokens.get(j).is_some_and(|t| t == "{") {
                stack.push(opened);
            }
            i = if tokens.get(j).is_some_and(|t| t == "}") { j } else { j + 1 };
        }
    }
}

fn list(names: &BTreeSet<String>) -> String {
    let quoted: Vec<String> = names.iter().map(|name| format!("{name:?}")).collect();
    format!("&[{}]", quoted.join(", "))
}

pub fn run() -> Result {
    let mut paths = Vec::new();
    files(Path::new(LIBRARY), &mut paths)?;
    paths.sort();
    let mut scanner =
        Scanner { packages: BTreeMap::new(), plain_name: Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").unwrap() };
    for path in &paths {
        let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        scanner.scan(&tokens_of(&String::from_utf8_lossy(&bytes)));
    }

    // As a reader looks a package up: case aside first.
    let mut packages: Vec<(&String, &Package)> = scanner.packages.iter().collect();
    packages.sort_by_key(|(name, _)| (name.to_lowercase(), (*name).clone()));
    let entries: Vec<String> = packages
        .iter()
        .map(|(name, package)| {
            format!(
                "    LibraryPackage {{ name: {name:?}, names: {}, imports: {} }},\n",
                list(&package.names),
                list(&package.imports)
            )
        })
        .collect();
    write(
        OUT,
        &format!(
            "// Generated by `cargo xtask library-names` from OMG's standard library, release\n\
             // 2026-09; do not edit. Run it again to regenerate this file.\n\
             \n\
             /// A package of the standard library: its name, the names it declares, and the\n\
             /// packages it imports publicly, so that a view may name a library element, such as a\n\
             /// part typed `SpatialItems::SpatialItem`, without ortho loading the library.\n\
             pub struct LibraryPackage {{\n    pub name: &'static str,\n    pub names: &'static [&'static str],\n    \
             pub imports: &'static [&'static str],\n}}\n\
             \n\
             #[rustfmt::skip]\n\
             pub const LIBRARY: &[LibraryPackage] = &[\n{}];\n",
            entries.concat()
        ),
    )?;
    let names: usize = scanner.packages.values().map(|p| p.names.len()).sum();
    println!("Wrote {OUT}: {} packages, {names} names", scanner.packages.len());
    Ok(())
}
