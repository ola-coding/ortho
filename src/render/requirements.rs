//! A table of requirements as Markdown, laid out as a requirements specification
//! is: a section for each requirement, nested ones under the one they are in, with
//! its ID, statement, rationale and status in a box of its own, then a table of what
//! satisfies each, and what verifies each once the model has verification cases: its
//! traceability. GitHub renders it, and lists its headings as an outline.
use crate::diagrams::requirements::Row;
use crate::parser::ast::NodeId;
use std::collections::HashMap;

/// What a Markdown file of requirements says besides its rows.
pub struct Specification<'a> {
    /// The view's name.
    pub heading: &'a str,
    /// The file that declares the view, by name and as the Markdown file links to it.
    pub source: (&'a str, String),
    /// The view's own `doc`, a paragraph each.
    pub introduction: Vec<String>,
    /// Whether the model has a verification case, so that the table says what
    /// verifies each requirement.
    pub verification: bool,
}

/// The Markdown of a table of requirements. `link` gives where an element is
/// declared, as the Markdown file links to it: `../physical.sysml#L159`.
pub fn render_requirements_markdown(
    specification: &Specification,
    rows: &[Row],
    link: &dyn Fn(NodeId) -> Option<String>,
) -> String {
    let mut anchors = Anchors::default();
    anchors.add(specification.heading);
    let headings: Vec<String> = rows.iter().map(plain_heading).collect();
    let anchor_of: HashMap<NodeId, String> =
        rows.iter().zip(&headings).map(|(row, heading)| (row.element, anchors.add(heading))).collect();
    let label = |row: &Row| row.id.as_deref().or(row.name.as_deref()).unwrap_or("requirement").to_string();
    let cross_reference = |element: NodeId| -> String {
        let row = rows.iter().find(|row| row.element == element);
        match (row, anchor_of.get(&element)) {
            (Some(row), Some(anchor)) => format!("[{}](#{anchor})", escape(&label(row))),
            _ => String::new(),
        }
    };

    let (name, source) = &specification.source;
    let mut out = format!("# {}\n\n", escape(specification.heading));
    out.push_str(&format!(
        "<sub>Rendered by ortho from [{}]({source}). Edit the model, not this file.</sub>\n\n",
        escape(name)
    ));
    for paragraph in &specification.introduction {
        out.push_str(&format!("{}\n\n", paragraph_text(paragraph)));
    }
    let unsatisfied: Vec<String> =
        rows.iter().filter(|row| !row.is_satisfied()).map(|row| cross_reference(row.element)).collect();
    let unverified: Vec<String> =
        rows.iter().filter(|row| row.verified_by.is_empty()).map(|row| cross_reference(row.element)).collect();
    let mut summary = summary_sentence(rows.len(), &unsatisfied, specification.verification.then_some(&unverified));
    for (status, words) in OPEN_STATUSES {
        let open: Vec<String> = rows
            .iter()
            .filter(|row| row.status.as_deref() == Some(status))
            .map(|row| cross_reference(row.element))
            .collect();
        if !open.is_empty() {
            summary.push_str(&format!(" {words}: {}.", open.join(", ")));
        }
    }
    out.push_str(&format!("{summary}\n\n"));

    for (row, heading) in rows.iter().zip(&headings) {
        let level = "#".repeat((row.depth + 2).min(6));
        out.push_str(&format!("{level} {}\n\n", escape(heading)));
        out.push_str(&requirement_box(row));
    }

    if !rows.is_empty() {
        if specification.verification {
            out.push_str(
                "## Traceability\n\n| ID | Requirement | Satisfied by | Verified by |\n|:---|:---|:---|:---|\n",
            );
        } else {
            out.push_str("## Traceability\n\n| ID | Requirement | Satisfied by |\n|:---|:---|:---|\n");
        }
        for row in rows {
            let anchor = &anchor_of[&row.element];
            let id = row.id.as_deref().map(|id| format!("[{}](#{anchor})", escape(id))).unwrap_or_default();
            let indent = "&emsp;".repeat(row.depth);
            let name = row.name.as_deref().map(escape).unwrap_or_default();
            let name = if row.id.is_none() { format!("[{name}](#{anchor})") } else { name };
            let satisfied_by = if !row.satisfied_by.is_empty() {
                let by = row.satisfied_by.iter().map(|(satisfier, by)| match link(*satisfier) {
                    Some(target) => format!("[{}]({target})", escape(by)),
                    None => escape(by),
                });
                by.collect::<Vec<_>>().join(", ")
            } else if let Some(above) = row.covered_by {
                format!("through {}", cross_reference(above))
            } else {
                "—".to_string()
            };
            out.push_str(&format!("| {id} | {indent}{name} | {satisfied_by} |"));
            if specification.verification {
                out.push_str(&format!(" {} |", verified_by(row, link)));
            }
            out.push('\n');
        }
    }
    out.truncate(out.trim_end().len());
    out.push('\n');
    out
}

/// A requirement's box: a table with its ID and statement, its `text`, and its
/// rationale when it has one, under their headings. A cell holds no paragraphs, so
/// they are broken by lines.
fn requirement_box(row: &Row) -> String {
    let cell = |paragraphs: &[String]| {
        let text = paragraphs.iter().map(|paragraph| escape(paragraph)).collect::<Vec<_>>().join("<br><br>");
        if text.is_empty() { "—".to_string() } else { text }
    };
    let id = row.id.as_deref().map_or("—".to_string(), escape);
    let mut columns = vec![("ID", id), ("Statement", cell(&row.text))];
    if !row.rationale.is_empty() {
        columns.push(("Rationale", cell(&row.rationale)));
    }
    columns.push(("Status", row.status.as_deref().map_or("—".to_string(), status_words)));
    let line = |cells: Vec<&str>| format!("| {} |\n", cells.join(" | "));
    let mut table = line(columns.iter().map(|(heading, _)| *heading).collect());
    table.push_str(&format!("|{}\n", ":---|".repeat(columns.len())));
    table.push_str(&line(columns.iter().map(|(_, value)| value.as_str()).collect()));
    table.push('\n');
    table
}

/// What the console says of a table of requirements: `8 requirement(s), 7 satisfied`,
/// and `, 6 verified` once the model has a verification case.
pub fn requirements_summary(rows: &[Row], verification: bool) -> String {
    let satisfied = rows.iter().filter(|row| row.is_satisfied()).count();
    let mut summary = format!("{} requirement(s), {satisfied} satisfied", rows.len());
    if verification {
        let verified = rows.iter().filter(|row| !row.verified_by.is_empty()).count();
        summary.push_str(&format!(", {verified} verified"));
    }
    summary
}

/// The statuses of ModelingMetadata's StatusKind that leave work open on a
/// requirement, as the summary lists them.
const OPEN_STATUSES: [(&str, &str); 3] =
    [("tbd", "To be determined"), ("tbr", "To be resolved"), ("tbc", "To be confirmed")];

/// A status as engineers write it: `TBD`, `TBR` and `TBC` in capitals, `Open`,
/// `Done` and `Closed` as words. Any other is shown as written.
fn status_words(status: &str) -> String {
    match status {
        "tbd" | "tbr" | "tbc" => status.to_uppercase(),
        "open" => "Open".to_string(),
        "done" => "Done".to_string(),
        "closed" => "Closed".to_string(),
        other => escape(other),
    }
}

/// A verification method as engineers name it: Inspection, Analysis, Demonstration, Test.
fn method_words(kind: &str) -> String {
    match kind {
        "inspect" => "Inspection".to_string(),
        "analyze" => "Analysis".to_string(),
        "demo" => "Demonstration".to_string(),
        "test" => "Test".to_string(),
        other => escape(other),
    }
}

/// The verification cases of a requirement, each linked, with its methods in brackets.
fn verified_by(row: &Row, link: &dyn Fn(NodeId) -> Option<String>) -> String {
    if row.verified_by.is_empty() {
        return "—".to_string();
    }
    let cases = row.verified_by.iter().map(|verification| {
        let name = escape(&verification.name);
        let mut case = match link(verification.case) {
            Some(target) => format!("[{name}]({target})"),
            None => name,
        };
        if !verification.methods.is_empty() {
            let methods: Vec<String> = verification.methods.iter().map(|kind| method_words(kind)).collect();
            case.push_str(&format!(" ({})", methods.join(", ")));
        }
        case
    });
    cases.collect::<Vec<_>>().join(", ")
}

/// How many requirements there are, how many are satisfied, and verified once the
/// model has verification cases, with those that are not, linked: `15 requirements,
/// all satisfied, 14 verified. Not verified: REQ-7.`
fn summary_sentence(count: usize, unsatisfied: &[String], unverified: Option<&Vec<String>>) -> String {
    if count == 0 {
        return "No requirements.".to_string();
    }
    let clause = |missing: usize, word: &str| {
        let done = count - missing;
        match (count, done) {
            (1, 1) => word.to_string(),
            (1, _) => format!("not {word}"),
            (_, 0) => format!("none {word}"),
            _ if done == count => format!("all {word}"),
            _ => format!("{done} {word}"),
        }
    };
    let noun = if count == 1 { "1 requirement".to_string() } else { format!("{count} requirements") };
    let mut sentence = format!("{noun}, {}", clause(unsatisfied.len(), "satisfied"));
    if let Some(unverified) = unverified {
        sentence.push_str(&format!(", {}", clause(unverified.len(), "verified")));
    }
    sentence.push('.');
    let partly = |missing: usize| missing > 0 && missing < count;
    if partly(unsatisfied.len()) {
        sentence.push_str(&format!(" Not satisfied: {}.", unsatisfied.join(", ")));
    }
    if let Some(unverified) = unverified.filter(|unverified| partly(unverified.len())) {
        sentence.push_str(&format!(" Not verified: {}.", unverified.join(", ")));
    }
    sentence
}

/// A requirement's heading, as GitHub also reads it to make its anchor: its name,
/// or its ID when it has none, since its box shows its ID.
fn plain_heading(row: &Row) -> String {
    row.name.as_deref().or(row.id.as_deref()).unwrap_or("requirement").to_string()
}

/// The anchors GitHub gives a file's headings, in order: each in lower case, with
/// spaces as hyphens and other punctuation but `-` and `_` left out, and a heading
/// like one before it numbered, `-1`, `-2`.
#[derive(Default)]
struct Anchors {
    taken: HashMap<String, usize>,
}

impl Anchors {
    fn add(&mut self, heading: &str) -> String {
        let slug: String = heading
            .to_lowercase()
            .chars()
            .filter_map(|c| match c {
                ' ' => Some('-'),
                '-' | '_' => Some(c),
                c if c.is_alphanumeric() => Some(c),
                _ => None,
            })
            .collect();
        let count = self.taken.entry(slug.clone()).or_insert(0);
        let anchor = if *count == 0 { slug.clone() } else { format!("{slug}-{count}") };
        *count += 1;
        anchor
    }
}

/// A paragraph of text as Markdown that shows it as written.
fn paragraph_text(text: &str) -> String {
    let escaped = escape(text);
    // A paragraph that starts as a heading or a list would read as one: `# of cups`, `- 5 °C`, `1. Fill`.
    if escaped.starts_with(['#', '-', '+']) {
        return format!("\\{escaped}");
    }
    let digits = escaped.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && escaped[digits..].starts_with(['.', ')']) {
        return format!("{}\\{}", &escaped[..digits], &escaped[digits..]);
    }
    escaped
}

/// Text as Markdown that shows it as written, its markup characters escaped.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '|') {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}
