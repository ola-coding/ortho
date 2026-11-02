use super::number::{Js, hypot};
use super::text_metrics::{Weight, measure_text};
use super::theme::{Theme, radius_attrs};
use crate::layout::{LaidOutDiagram, LaidOutEdge, LaidOutNode, PORT_LABEL_FONT, PORT_SIZE, Point};
use crate::model::graph::{
    BOX_HEADER_HEIGHT as HEADER_HEIGHT, COMPARTMENT_LINE_HEIGHT as LINE_HEIGHT, COMPARTMENT_PADDING,
    COMPARTMENT_TITLE_HEIGHT, EdgeKind, NodeShape, USAGE_RADIUS,
};

pub const PAD_X: f64 = 24.0;
pub const PAD_TOP: f64 = 48.0;
/// Room below the drawing, clear of the frame's corner compartment.
const PAD_BOTTOM: f64 = 30.0;
pub const FONT: &str = "Arial, Helvetica, Liberation Sans, DejaVu Sans, sans-serif";

/// A port is a usage too, at a port's size.
const PORT_RADIUS: f64 = 2.0;

pub fn escape_xml(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '«' => escaped.push_str("&#171;"),
            '»' => escaped.push_str("&#187;"),
            c => escaped.push(c),
        }
    }
    escaped
}

/// What the frame says: the view's name in its tab, and the model file that declares
/// the view in its bottom-right corner (everything-as-code: every SVG names the file
/// that generated it).
pub struct DiagramTitle {
    /// The view's name; empty for an unnamed view.
    pub heading: String,
    /// Its view definition, as written: `GeneralView`. The tab shows it only for an unnamed view.
    pub definition: Option<String>,
    pub source: String,
}

/// The frame's inset from the page, and its corners: a view is a usage.
const FRAME_INSET: f64 = 8.0;
const FRAME_RADIUS: f64 = 10.0;
const TAB_HEIGHT: f64 = 22.0;
const TAB_RADIUS: f64 = 8.0;
const INFO_HEIGHT: f64 = 15.0;
const INFO_RADIUS: f64 = 6.0;

/// The view frame (§8.2.3.26): a rounded border, a tab in its top-left corner reading
/// «view» and the view's name, and an info compartment in its bottom-right corner
/// holding the source file. The view's definition is optional there, and stands in
/// only for a missing name.
pub fn render_frame(width: f64, height: f64, title: &DiagramTitle, t: &Theme) -> String {
    let Some(frame_fill) = t.frame_fill else {
        return String::new();
    };
    let left = FRAME_INSET;
    let top = FRAME_INSET;
    let right = width - FRAME_INSET;
    let bottom = height - FRAME_INSET;
    let name = if title.heading.is_empty() { String::new() } else { format!(" {}", title.heading) };
    let definition = match &title.definition {
        Some(definition) if title.heading.is_empty() && !definition.is_empty() => format!(" : {definition}"),
        _ => String::new(),
    };
    let tab_right = left
        + 22.0
        + measure_text("«view»", 11.0, Weight::Regular)
        + measure_text(&name, 11.0, Weight::Bold)
        + measure_text(&definition, 11.0, Weight::Regular);
    let tab_bottom = top + TAB_HEIGHT;
    let info_left = right - measure_text(&title.source, 9.0, Weight::Regular) - 16.0;
    let info_top = bottom - INFO_HEIGHT;
    format!(
        r#"
  <rect x="{left}" y="{top}" width="{frame_width}" height="{frame_height}"{radius} fill="{frame_fill}" {line} />
  <path d="M{left},{tab_bottom} L{left},{top_arc} A{fr},{fr} 0 0 1 {left_arc},{top} L{tab_right},{top} L{tab_right},{tab_arc} A{tr},{tr} 0 0 1 {tab_end},{tab_bottom} Z"
        fill="{tint}" {line} />
  <text x="{text_x}" y="{text_y}" font-size="11" fill="{text}"><tspan font-style="italic" fill="{keyword}">&#171;view&#187;</tspan><tspan font-weight="600">{name}</tspan>{definition}</text>
  <path d="M{info_left},{bottom} L{info_left},{info_arc} A{ir},{ir} 0 0 1 {info_end},{info_top} L{right},{info_top} L{right},{bottom_arc} A{fr},{fr} 0 0 1 {corner_x},{bottom} Z"
        fill="{tint}" {line} />
  <text x="{source_x}" y="{source_y}" text-anchor="end" font-size="9" fill="{provenance}">{source}</text>"#,
        left = Js(left),
        top = Js(top),
        right = Js(right),
        bottom = Js(bottom),
        frame_width = Js(right - left),
        frame_height = Js(bottom - top),
        radius = radius_attrs(FRAME_RADIUS),
        line = format!(r#"stroke="{}" stroke-width="1.25""#, t.box_stroke),
        tint = t.name_fill.unwrap_or(t.fill),
        tab_bottom = Js(tab_bottom),
        top_arc = Js(top + FRAME_RADIUS),
        fr = Js(FRAME_RADIUS),
        left_arc = Js(left + FRAME_RADIUS),
        tab_right = Js(tab_right),
        tab_arc = Js(tab_bottom - TAB_RADIUS),
        tr = Js(TAB_RADIUS),
        tab_end = Js(tab_right - TAB_RADIUS),
        text_x = Js(left + 10.0),
        text_y = Js(top + 15.0),
        text = t.text,
        keyword = t.keyword,
        name = escape_xml(&name),
        definition = escape_xml(&definition),
        info_left = Js(info_left),
        info_arc = Js(info_top + INFO_RADIUS),
        ir = Js(INFO_RADIUS),
        info_end = Js(info_left + INFO_RADIUS),
        info_top = Js(info_top),
        bottom_arc = Js(bottom - FRAME_RADIUS),
        corner_x = Js(right - FRAME_RADIUS),
        source_x = Js(right - 8.0),
        source_y = Js(bottom - 4.0),
        provenance = t.provenance_text,
        source = escape_xml(&title.source),
    )
}

/// The SVG's `<title>`: the view, and the file it was generated from.
pub fn title_text(title: &DiagramTitle) -> String {
    let name = if title.heading.is_empty() { title.definition.as_deref().unwrap_or("") } else { &title.heading };
    format!("{name} (generated from {})", title.source)
}

fn render_node(n: &LaidOutNode, t: &Theme) -> String {
    match n.shape {
        NodeShape::Actor => render_actor_node(n, t),
        NodeShape::Package => render_package_node(n, t),
        NodeShape::Definition | NodeShape::Usage => render_box_node(n, t),
    }
}

fn render_actor_node(n: &LaidOutNode, t: &Theme) -> String {
    let cx = n.x + PAD_X + n.width / 2.0;
    let top = n.y + PAD_TOP;
    format!(
        r#"
    <g stroke="{stroke}" stroke-width="1.5" fill="none">
      <circle cx="{cx}" cy="{head}" r="8" fill="{actor_fill}" />
      <line x1="{cx}" y1="{neck}" x2="{cx}" y2="{hip}" />
      <line x1="{arm_left}" y1="{arm}" x2="{arm_right}" y2="{arm}" />
      <line x1="{cx}" y1="{hip}" x2="{leg_left}" y2="{foot}" />
      <line x1="{cx}" y1="{hip}" x2="{leg_right}" y2="{foot}" />
      <text x="{cx}" y="{label}" text-anchor="middle" stroke="none"
          font-size="11" fill="{text}">{name}</text>
    </g>"#,
        stroke = t.stroke,
        actor_fill = t.actor_fill,
        text = t.text,
        cx = Js(cx),
        head = Js(top + 9.0),
        neck = Js(top + 17.0),
        hip = Js(top + 36.0),
        arm_left = Js(cx - 13.0),
        arm = Js(top + 24.0),
        arm_right = Js(cx + 13.0),
        leg_left = Js(cx - 11.0),
        foot = Js(top + 52.0),
        leg_right = Js(cx + 11.0),
        label = Js(top + 68.0),
        name = escape_xml(&n.name),
    )
}

/// A package (§8.2.3.5): a tabbed box. One holding others carries its name in its tab,
/// above what it holds; one holding nothing drawn carries it inside, under an empty tab.
fn render_package_node(n: &LaidOutNode, t: &Theme) -> String {
    let x = n.x + PAD_X;
    let y = n.y + PAD_TOP;
    let name_width = measure_text(&n.name, 11.0, Weight::Bold);
    let (tab_width, tab_height) =
        if n.has_children { (name_width + 16.0, 18.0) } else { ((name_width + 20.0).clamp(46.0, 90.0), 12.0) };
    let tint = t.name_fill.unwrap_or(t.fill);
    let name = if n.has_children {
        format!(
            r#"<text x="{x}" y="{y}" font-size="11" font-weight="600" fill="{text}">{name}</text>"#,
            x = Js(x + 8.0),
            y = Js(y + 13.0),
            text = t.text,
            name = escape_xml(&n.name),
        )
    } else {
        format!(
            r#"<text x="{x}" y="{y}" text-anchor="middle"
          font-size="12" font-weight="600" fill="{text}">{name}</text>"#,
            x = Js(x + n.width / 2.0),
            y = Js(y + 12.0 + (n.height - 12.0) / 2.0 + 4.0),
            text = t.text,
            name = escape_xml(&n.name),
        )
    };
    format!(
        r#"
    <g><rect x="{x}" y="{y}" width="{tab_width}" height="{tab_height}"
          fill="{tint}" stroke="{stroke}" stroke-width="1.25" />
      <rect x="{x}" y="{body_y}" width="{width}" height="{body_height}"
          fill="{body}" stroke="{stroke}" stroke-width="1.25" />{gloss}
      {name}</g>"#,
        x = Js(x),
        y = Js(y),
        tab_width = Js(tab_width),
        tab_height = Js(tab_height),
        stroke = t.box_stroke,
        body_y = Js(y + tab_height),
        width = Js(n.width),
        body_height = Js(n.height - tab_height),
        body = if n.has_children { t.fill } else { tint },
        gloss = t.gloss_over(x, y + tab_height, n.width, n.height - tab_height, 0.0),
    )
}

/// A name compartment's tint, up to the box's outline.
fn name_band(x: f64, y: f64, width: f64, radius: f64, fill: &str) -> String {
    let inset = 0.5;
    let r = (radius - inset).max(0.0);
    let left = x + inset;
    let right = x + width - inset;
    let top = y + inset;
    let bottom = y + HEADER_HEIGHT;
    let corner = |to_x: f64, to_y: f64| {
        if r > 0.0 { format!("A{r},{r} 0 0 1 {},{} ", Js(to_x), Js(to_y), r = Js(r)) } else { String::new() }
    };
    format!(
        r#"<path d="M{left},{bottom} L{left},{start} {first}L{end},{top} {second}L{right},{bottom} Z" fill="{fill}" />"#,
        left = Js(left),
        bottom = Js(bottom),
        start = Js(top + r),
        first = corner(left + r, top),
        end = Js(right - r),
        top = Js(top),
        second = corner(right, top + r),
        right = Js(right),
    )
}

/// A definition or a usage: a box with square or rounded corners, its keyword above
/// its name, compartments below and ports on its border.
fn render_box_node(n: &LaidOutNode, t: &Theme) -> String {
    let x = n.x + PAD_X;
    let y = n.y + PAD_TOP;
    let radius = if n.shape == NodeShape::Usage { USAGE_RADIUS } else { 0.0 };
    let mut parts: Vec<String> = Vec::new();
    let center = Js(x + n.width / 2.0);
    let name = escape_xml(&n.name);

    // The name compartment is tinted, and it is the whole box when the box holds
    // nothing else.
    let holds_more = !n.compartments.is_empty() || n.has_children;
    let body = if n.has_children { t.container_fill } else { t.fill };
    parts.push(format!(
        r#"<rect x="{x}" y="{y}" width="{width}" height="{height}"{radius}{lift}
          fill="{fill}" stroke="{stroke}" stroke-width="1.25" />"#,
        x = Js(x),
        y = Js(y),
        width = Js(n.width),
        height = Js(n.height),
        radius = radius_attrs(radius),
        lift = t.lift(),
        fill = if holds_more { body } else { t.name_fill.unwrap_or(body) },
        stroke = t.box_stroke,
    ));
    if holds_more && let Some(name_fill) = t.name_fill {
        parts.push(name_band(x, y, n.width, radius, name_fill));
    }
    let gloss = t.gloss_over(x, y, n.width, n.height, radius);
    if !gloss.is_empty() {
        parts.push(gloss.trim().to_string());
    }

    // The keyword and name are centred in the name compartment.
    let header = if holds_more { HEADER_HEIGHT } else { n.height };
    let top = y + (header - HEADER_HEIGHT) / 2.0;
    if !n.keyword.is_empty() {
        parts.push(format!(
            r#"<text x="{center}" y="{y}" text-anchor="middle"
          font-size="9" font-style="italic" fill="{keyword}">&#171;{word}&#187;</text>"#,
            y = Js(top + 13.0),
            keyword = t.keyword,
            word = escape_xml(&n.keyword),
        ));
        parts.push(format!(
            r#"<text x="{center}" y="{y}" text-anchor="middle"
          font-size="12" font-weight="600" fill="{text}">{name}</text>"#,
            y = Js(top + 26.0),
            text = t.text,
        ));
    } else {
        parts.push(format!(
            r#"<text x="{center}" y="{y}" text-anchor="middle"
          font-size="12" font-weight="600" fill="{text}">{name}</text>"#,
            y = Js(top + 21.0),
            text = t.text,
        ));
    }

    let divider = |at: f64| {
        format!(
            r#"<line x1="{x1}" y1="{at}" x2="{x2}" y2="{at}"
          stroke="{stroke}" stroke-width="1" />"#,
            x1 = Js(x),
            x2 = Js(x + n.width),
            at = Js(at),
            stroke = t.box_stroke,
        )
    };
    let mut compartment_top = y + HEADER_HEIGHT;
    for compartment in &n.compartments {
        parts.push(divider(compartment_top));
        let mut lines_top = compartment_top;
        if let Some(title) = &compartment.title {
            parts.push(format!(
                r#"<text x="{center}" y="{y}" text-anchor="middle"
          font-size="9" font-style="italic" fill="{keyword}">{title}</text>"#,
                y = Js(compartment_top + 11.0),
                keyword = t.keyword,
                title = escape_xml(title),
            ));
            lines_top += COMPARTMENT_TITLE_HEIGHT;
        }
        for (i, line) in compartment.lines.iter().enumerate() {
            parts.push(format!(
                r#"<text x="{x}" y="{y}"
          font-size="11" fill="{text}">{line}</text>"#,
                x = Js(x + 8.0),
                y = Js(lines_top + 12.0 + i as f64 * LINE_HEIGHT),
                text = t.text,
                line = escape_xml(line),
            ));
        }
        compartment_top += COMPARTMENT_PADDING
            + if compartment.title.is_some() { COMPARTMENT_TITLE_HEIGHT } else { 0.0 }
            + compartment.lines.len() as f64 * LINE_HEIGHT;
    }
    if n.has_children {
        parts.push(divider(compartment_top));
    }

    for port in &n.ports {
        parts.push(format!(
            r#"<rect x="{x}" y="{y}" width="{size}" height="{size}"{radius}
          fill="{fill}" stroke="{stroke}" stroke-width="1.25" />"#,
            x = Js(x + port.x),
            y = Js(y + port.y),
            size = Js(PORT_SIZE),
            radius = radius_attrs(PORT_RADIUS),
            fill = t.port_fill,
            stroke = t.box_stroke,
        ));
    }

    format!("\n    <g>{}</g>", parts.join("\n      "))
}

/// Port labels are painted after every edge, so a line passing a port never strikes
/// through its name. ELK places them (outside the box, beside the port, on whichever
/// side avoids a neighbour); the fallback only covers a port ELK returned no label
/// position for.
fn render_port_labels(n: &LaidOutNode, t: &Theme) -> String {
    let x = n.x + PAD_X;
    let y = n.y + PAD_TOP;
    let halo = format!(
        r#"font-size="{font}" fill="{text}" paint-order="stroke" stroke="{halo}" stroke-width="2.5""#,
        font = Js(PORT_LABEL_FONT),
        text = t.text,
        halo = t.halo,
    );
    let mut labels = String::new();
    for port in &n.ports {
        let px = x + port.x;
        let py = y + port.y;
        let label = escape_xml(&port.label);
        if let Some((label_x, label_y)) = port.label_at {
            labels.push_str(&format!(
                "\n    <text x=\"{}\" y=\"{}\" {halo}>{label}</text>",
                Js(px + label_x),
                Js(py + label_y + PORT_LABEL_FONT),
            ));
        } else {
            let label_width = measure_text(&port.label, PORT_LABEL_FONT, Weight::Regular);
            let rightwards = port.x + PORT_SIZE + 6.0 + label_width <= n.width;
            let label_x = if rightwards { px + PORT_SIZE + 4.0 } else { px - 4.0 };
            labels.push_str(&format!(
                "\n    <text x=\"{}\" y=\"{}\"{anchor} {halo}>{label}</text>",
                Js(label_x),
                Js(py + PORT_SIZE / 2.0 + 3.0),
                anchor = if rightwards { "" } else { " text-anchor=\"end\"" },
            ));
        }
    }
    labels
}

fn point_at_fraction(points: &[Point], fraction: f64) -> (f64, f64) {
    let lengths: Vec<f64> = points.windows(2).map(|p| hypot(p[1].x - p[0].x, p[1].y - p[0].y)).collect();
    let total = lengths.iter().fold(0.0, |sum, length| sum + length);
    let mut remaining = total * fraction;
    for (i, length) in lengths.iter().enumerate() {
        if remaining <= *length {
            let t = if *length == 0.0 { 0.0 } else { remaining / length };
            let (from, to) = (&points[i], &points[i + 1]);
            return (from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
        }
        remaining -= length;
    }
    let last = &points[points.len() - 1];
    (last.x, last.y)
}

fn render_edge(e: &LaidOutEdge, t: &Theme) -> String {
    if e.points.len() < 2 {
        return String::new();
    }
    let points: Vec<String> = e.points.iter().map(|p| format!("{},{}", Js(p.x + PAD_X), Js(p.y + PAD_TOP))).collect();
    // «include», «import» and a dependency carry an open arrowhead; the last two are
    // dashed (§8.2.3.3, §8.2.3.5), an «include» solid (§8.2.3.25). A branch of the
    // function tree is owned by its parent: a filled diamond at the parent's end
    // (§8.2.3.6). Every other line is plain: a connection, or a use case's line.
    let dashed = matches!(e.kind, EdgeKind::Import | EdgeKind::Dependency);
    let dash = if dashed { r#" stroke-dasharray="6 4""# } else { "" };
    let markers = if dashed || e.kind == EdgeKind::Include {
        r#" marker-end="url(#openArrow)""#
    } else if e.kind == EdgeKind::Decomposition {
        r#" marker-start="url(#diamond)""#
    } else {
        ""
    };
    let mut svg = format!(
        r#"
    <polyline points="{points}" fill="none" stroke="{stroke}" stroke-width="1.25"{dash}{markers} />"#,
        points = points.join(" "),
        stroke = t.stroke,
    );
    if let Some(label) = &e.label {
        // A label ELK placed is centred in the box ELK reserved for it. Any other sits
        // just above the line's midpoint.
        let (text_x, text_y) = match &e.label_box {
            Some(b) => (b.x + b.width / 2.0 + PAD_X, b.y + b.height / 2.0 + 3.0 + PAD_TOP),
            None => {
                let (x, y) = point_at_fraction(&e.points, 0.5);
                (x + PAD_X, y + PAD_TOP - 5.0)
            }
        };
        svg.push_str(&format!(
            r#"
    <text x="{x}" y="{y}" text-anchor="middle" font-size="9"
          fill="{fill}" paint-order="stroke" stroke="{halo}" stroke-width="3">{label}</text>"#,
            x = Js(text_x),
            y = Js(text_y),
            fill = t.edge_label,
            halo = t.halo,
            label = escape_xml(label),
        ));
    }
    svg
}

pub fn render_svg(diagram: &LaidOutDiagram, title: &DiagramTitle, t: &Theme) -> String {
    let width = diagram.width + PAD_X * 2.0;
    let height = diagram.height + PAD_TOP + PAD_BOTTOM;

    let nodes: String = diagram.nodes.iter().map(|n| render_node(n, t)).collect();
    let edges: String = diagram.edges.iter().map(|e| render_edge(e, t)).collect();
    let port_labels: String = diagram.nodes.iter().map(|n| render_port_labels(n, t)).collect();

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}"
     viewBox="0 0 {w} {h}" font-family="{FONT}">
  <title>{title_text}</title>
  <defs>{defs}
    <marker id="openArrow" markerWidth="12" markerHeight="12" refX="10" refY="5"
            orient="auto" markerUnits="userSpaceOnUse">
      <path d="M0,0 L10,5 L0,10" fill="none" stroke="{stroke}" stroke-width="1.25" />
    </marker>
    <marker id="diamond" markerWidth="12" markerHeight="8" refX="0" refY="4"
            orient="auto" markerUnits="userSpaceOnUse">
      <path d="M0,4 L6,0 L12,4 L6,8 z" fill="{stroke}" />
    </marker>
  </defs>
  {page}{frame}
  {nodes}
  {edges}{port_labels}
</svg>
"##,
        w = Js(width),
        h = Js(height),
        title_text = escape_xml(&title_text(title)),
        defs = t.defs,
        stroke = t.stroke,
        page = page(width, height, t),
        frame = render_frame(width, height, title, t),
        port_labels = if port_labels.is_empty() { String::new() } else { format!("\n  {port_labels}") },
    )
}

/// The page behind a diagram, when the theme has one.
pub(super) fn page(width: f64, height: f64, t: &Theme) -> String {
    match t.page {
        Some(fill) => format!(r#"<rect x="0" y="0" width="{}" height="{}" fill="{fill}" />"#, Js(width), Js(height)),
        None => String::new(),
    }
}
