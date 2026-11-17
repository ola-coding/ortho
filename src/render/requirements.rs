//! A table of requirements as Markdown, laid out as a requirements specification
//! is: a section for each requirement, with its text and rationale, nested ones
//! under the one they are in, then a table of what satisfies each, its traceability.
//! GitHub renders it, and lists its headings as an outline.
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
}

/// The Markdown of a table of requirements. `link` gives where an element is
/// declared, as the Markdown file links to it: `../requirements.sysml#L8`.
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
    out.push_str(&format!("{}\n\n", summary_sentence(rows.len(), &unsatisfied)));

    for (row, heading) in rows.iter().zip(&headings) {
        let level = "#".repeat((row.depth + 2).min(6));
        let linked = match (&row.id, link(row.element)) {
            (Some(id), Some(target)) => format!("[{}]({target})", escape(id)),
            (Some(id), None) => escape(id),
            (None, _) => String::new(),
        };
        let name = row.name.as_deref().map(escape).unwrap_or_default();
        let title = [linked, name].into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" ");
        let title = if title.is_empty() { escape(heading) } else { title };
        out.push_str(&format!("{level} {title}\n\n"));
        for paragraph in &row.text {
            out.push_str(&format!("{}\n\n", paragraph_text(paragraph)));
        }
        for rationale in &row.rationale {
            out.push_str(&format!("**Rationale:** {}\n\n", escape(rationale)));
        }
    }

    if !rows.is_empty() {
        out.push_str("## Traceability\n\n| ID | Requirement | Satisfied by |\n|:---|:---|:---|\n");
        for row in rows {
            let anchor = &anchor_of[&row.element];
            let id = row.id.as_deref().map(|id| format!("[{}](#{anchor})", escape(id))).unwrap_or_default();
            let indent = "&emsp;".repeat(row.depth);
            let name = row.name.as_deref().map(escape).unwrap_or_default();
            let name = if row.id.is_none() { format!("[{name}](#{anchor})") } else { name };
            let satisfied_by = if !row.satisfied_by.is_empty() {
                let by = row.satisfied_by.iter().map(|(satisfy, by)| match link(*satisfy) {
                    Some(target) => format!("[{}]({target})", escape(by)),
                    None => escape(by),
                });
                by.collect::<Vec<_>>().join(", ")
            } else if let Some(above) = row.covered_by {
                format!("through {}", cross_reference(above))
            } else {
                "—".to_string()
            };
            out.push_str(&format!("| {id} | {indent}{name} | {satisfied_by} |\n"));
        }
    }
    out.truncate(out.trim_end().len());
    out.push('\n');
    out
}

/// What the console says of a table of requirements: `8 requirement(s), 7 satisfied`.
pub fn requirements_summary(rows: &[Row]) -> String {
    let satisfied = rows.iter().filter(|row| row.is_satisfied()).count();
    format!("{} requirement(s), {satisfied} satisfied", rows.len())
}

/// How many requirements there are, and which are not satisfied, linked.
fn summary_sentence(count: usize, unsatisfied: &[String]) -> String {
    let satisfied = count - unsatisfied.len();
    match (count, satisfied) {
        (0, _) => "No requirements.".to_string(),
        (1, 1) => "1 requirement, satisfied.".to_string(),
        (1, _) => "1 requirement, not satisfied.".to_string(),
        (_, 0) => format!("{count} requirements, none satisfied."),
        _ if satisfied == count => format!("{count} requirements, all satisfied."),
        _ => format!("{count} requirements, {satisfied} satisfied. Not satisfied: {}.", unsatisfied.join(", ")),
    }
}

/// A requirement's heading as GitHub reads it to make its anchor: its ID and its name.
fn plain_heading(row: &Row) -> String {
    let parts = [row.id.as_deref(), row.name.as_deref()];
    let heading = parts.into_iter().flatten().collect::<Vec<_>>().join(" ");
    if heading.is_empty() { "requirement".to_string() } else { heading }
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
