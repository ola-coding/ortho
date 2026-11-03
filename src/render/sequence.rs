use super::number::Js;
use super::svg::{DiagramTitle, FONT, PAD_TOP, PAD_X, escape_xml, page, render_frame, title_text};
use super::text_metrics::{Weight, measure_text};
use super::theme::{Theme, radius_attrs};
use crate::diagrams::sequence::SequenceModel;
use crate::model::graph::USAGE_RADIUS;

/// A lifeline's head: a part, «part» above its name (§8.2.3.9).
const HEAD_HEIGHT: f64 = 36.0;
const MESSAGE_STEP: f64 = 40.0;
const SELF_LOOP_WIDTH: f64 = 40.0;

/// Sequence layout is deterministic (lifelines across the top, messages in order down
/// the page), so this renderer does its own layout instead of ELK. A message runs from
/// lifeline to lifeline (§8.2.3.9): SysML v2 draws no activation bars.
pub fn render_sequence_svg(model: &SequenceModel, title: &DiagramTitle, t: &Theme) -> String {
    let widths: Vec<f64> =
        model.lifelines.iter().map(|l| (measure_text(&l.label, 12.0, Weight::Bold) + 24.0).max(90.0)).collect();
    let column_of = |id: &str| model.lifelines.iter().position(|l| l.id == id).unwrap();

    // Per-gap spacing: start from head widths, then widen the gaps each message label
    // spans until the label fits.
    let mut gaps: Vec<f64> = widths.windows(2).map(|w| (w[0] / 2.0 + w[1] / 2.0 + 30.0).max(120.0)).collect();
    let mut right_extra: f64 = 0.0;
    for message in &model.messages {
        let a = column_of(&message.source_id);
        let b = column_of(&message.target_id);
        let label_width = measure_text(&message.label, 10.0, Weight::Regular);
        if a == b {
            let needed = SELF_LOOP_WIDTH + label_width + 24.0;
            if a == widths.len() - 1 {
                right_extra = right_extra.max(needed - widths[a] / 2.0);
            } else {
                gaps[a] = gaps[a].max(needed + widths[a + 1] / 2.0);
            }
        } else {
            let (lo, hi) = if a < b { (a, b) } else { (b, a) };
            let span = gaps[lo..hi].iter().fold(0.0, |sum, gap| sum + gap);
            let needed = label_width + 50.0;
            if span < needed {
                let grow = (needed - span) / (hi - lo) as f64;
                for gap in &mut gaps[lo..hi] {
                    *gap += grow;
                }
            }
        }
    }

    let mut centers = Vec::with_capacity(widths.len());
    let mut x = PAD_X + widths.first().copied().unwrap_or(90.0) / 2.0;
    for i in 0..widths.len() {
        centers.push(x);
        x += gaps.get(i).copied().unwrap_or(0.0);
    }
    let content_right = match (centers.last(), widths.last()) {
        (Some(center), Some(head)) => center + head / 2.0 + right_extra,
        _ => 200.0,
    };

    let head_top = PAD_TOP;
    let first_message_y = head_top + HEAD_HEIGHT + 36.0;
    let lifeline_bottom = first_message_y + model.messages.len() as f64 * MESSAGE_STEP;
    let width = content_right + PAD_X;
    // Lifelines end clear of the frame's corner compartment.
    let height = lifeline_bottom + 34.0;

    let mut parts = String::new();

    for (i, lifeline) in model.lifelines.iter().enumerate() {
        let (cx, head) = (centers[i], widths[i]);
        parts.push_str(&format!(
            r#"
    <g><rect x="{left}" y="{top}" width="{head_width}" height="{head_height}"{radius}{lift}
          fill="{fill}" stroke="{stroke}" stroke-width="1.25" />{gloss}
      <text x="{cx}" y="{keyword_y}" text-anchor="middle"
          font-size="9" font-style="italic" fill="{keyword}">&#171;part&#187;</text>
      <text x="{cx}" y="{name_y}" text-anchor="middle"
          font-size="12" font-weight="600" fill="{text}">{label}</text>
      <line x1="{cx}" y1="{line_top}" x2="{cx}" y2="{line_bottom}"
          stroke="{line}" stroke-width="1" stroke-dasharray="4 4" /></g>"#,
            left = Js(cx - head / 2.0),
            top = Js(head_top),
            head_width = Js(head),
            head_height = Js(HEAD_HEIGHT),
            radius = radius_attrs(USAGE_RADIUS),
            lift = t.lift(),
            fill = t.name_fill.unwrap_or(t.fill),
            stroke = t.box_stroke,
            gloss = t.gloss_over(cx - head / 2.0, head_top, head, HEAD_HEIGHT, USAGE_RADIUS),
            cx = Js(cx),
            keyword_y = Js(head_top + 15.0),
            keyword = t.keyword,
            name_y = Js(head_top + 28.0),
            text = t.text,
            label = escape_xml(&lifeline.label),
            line_top = Js(head_top + HEAD_HEIGHT),
            line_bottom = Js(lifeline_bottom),
            line = t.stroke,
        ));
    }

    for (i, message) in model.messages.iter().enumerate() {
        let y = first_message_y + i as f64 * MESSAGE_STEP;
        let source_x = centers[column_of(&message.source_id)];
        let target_x = centers[column_of(&message.target_id)];
        let label = escape_xml(&message.label);
        if message.source_id == message.target_id {
            let right = source_x + SELF_LOOP_WIDTH;
            parts.push_str(&format!(
                r#"
    <g><polyline points="{source},{y} {right},{y} {right},{below} {source},{below}"
          fill="none" stroke="{stroke}" stroke-width="1.25" marker-end="url(#msgArrow)" />
      <text x="{text_x}" y="{text_y}" font-size="10" fill="{text}">{label}</text></g>"#,
                source = Js(source_x),
                y = Js(y),
                right = Js(right),
                below = Js(y + 16.0),
                stroke = t.stroke,
                text_x = Js(right + 6.0),
                text_y = Js(y + 12.0),
                text = t.text,
            ));
        } else {
            parts.push_str(&format!(
                r#"
    <g><line x1="{source}" y1="{y}" x2="{target}" y2="{y}"
          stroke="{stroke}" stroke-width="1.25" marker-end="url(#msgArrow)" />
      <text x="{text_x}" y="{text_y}" text-anchor="middle" font-size="10"
          fill="{text}" paint-order="stroke" stroke="{halo}" stroke-width="3">{label}</text></g>"#,
                source = Js(source_x),
                y = Js(y),
                target = Js(target_x),
                stroke = t.stroke,
                text_x = Js((source_x + target_x) / 2.0),
                text_y = Js(y - 6.0),
                text = t.text,
                halo = t.halo,
            ));
        }
    }

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}"
     viewBox="0 0 {w} {h}" font-family="{FONT}">
  <title>{title_text}</title>
  <defs>{defs}
    <marker id="msgArrow" markerWidth="13" markerHeight="11" refX="11" refY="5" orient="auto"
            markerUnits="userSpaceOnUse">
      <path d="M1,1 L11,5 L1,9 z" fill="{halo}" stroke="{stroke}" stroke-width="1.25" />
    </marker>
  </defs>
  {page}{frame}
  {parts}
</svg>
"##,
        w = Js(width),
        h = Js(height),
        title_text = escape_xml(&title_text(title)),
        defs = t.defs,
        halo = t.halo,
        stroke = t.stroke,
        page = page(width, height, t),
        frame = render_frame(width, height, title, t),
    )
}
