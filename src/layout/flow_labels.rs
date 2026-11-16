//! Where the name of what flows along a connection goes (§8.2.3.16): beside the
//! flow's arrowhead on the line, in a place where nothing else is drawn. Placed
//! after the layout, so a diagram keeps the layout it has without its flows; when
//! a name finds no free place, the caller lays the diagram out again with room
//! kept for every one.
use super::{FLOW_ARROW, FLOW_LABEL_FONT, LabelBox, LaidOutDiagram, LaidOutNode, PORT_LABEL_FONT, PORT_SIZE, Point};
use crate::model::graph::{BOX_HEADER_HEIGHT, compartments_height};
use crate::render::text_metrics::{Weight, measure_text};

/// How tall a flow's name is.
const LABEL_HEIGHT: f64 = 12.0;
/// From the line to the near edge of the name: half the arrowhead's width, and a gap.
const OFFSET: f64 = FLOW_ARROW * 0.4 + 4.0;
/// Kept free around a name and an arrowhead.
const CLEAR: f64 = 3.0;
/// Where along a stretch of line an arrowhead is tried, the middle first.
const FRACTIONS: [f64; 7] = [0.5, 0.4, 0.6, 0.3, 0.7, 0.2, 0.8];

#[derive(Clone, Copy)]
struct Rect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl Rect {
    fn around(x: f64, y: f64, half_width: f64, half_height: f64) -> Rect {
        Rect { left: x - half_width, top: y - half_height, right: x + half_width, bottom: y + half_height }
    }

    fn grown(self, by: f64) -> Rect {
        Rect { left: self.left - by, top: self.top - by, right: self.right + by, bottom: self.bottom + by }
    }

    fn overlaps(&self, other: &Rect) -> bool {
        self.left < other.right && other.left < self.right && self.top < other.bottom && other.top < self.bottom
    }

    fn contains(&self, other: &Rect) -> bool {
        self.left <= other.left && other.right <= self.right && self.top <= other.top && other.bottom <= self.bottom
    }

    /// Whether the straight line from `a` to `b` passes through the rectangle. Lines
    /// on a diagram are horizontal or vertical, but any other is handled too.
    fn crossed_by(&self, a: Point, b: Point) -> bool {
        let line = Rect { left: a.x.min(b.x), top: a.y.min(b.y), right: a.x.max(b.x), bottom: a.y.max(b.y) };
        if !self.grown(0.01).overlaps(&line.grown(0.01)) {
            return false;
        }
        if a.x == b.x || a.y == b.y {
            return true;
        }
        // A slanted line: the corners lie on both sides of it.
        let side = |x: f64, y: f64| (b.x - a.x) * (y - a.y) - (b.y - a.y) * (x - a.x);
        let corners = [
            side(self.left, self.top),
            side(self.right, self.top),
            side(self.left, self.bottom),
            side(self.right, self.bottom),
        ];
        corners.iter().any(|c| *c >= 0.0) && corners.iter().any(|c| *c <= 0.0)
    }

    fn label_box(self) -> LabelBox {
        LabelBox { x: self.left, y: self.top, width: self.right - self.left, height: self.bottom - self.top }
    }
}

/// What a flow's name must keep clear of.
struct Drawn {
    /// Boxes, the header band of each container, ports, the names beside ports and
    /// lines, and every name and arrowhead placed so far.
    taken: Vec<Rect>,
    /// Each container's outline: a name is wholly inside one or wholly outside it.
    containers: Vec<Rect>,
    /// Every stretch of every line, with the index of the line it belongs to.
    stretches: Vec<(usize, Point, Point)>,
    bounds: Rect,
}

impl Drawn {
    fn of(diagram: &LaidOutDiagram) -> Drawn {
        let mut taken = Vec::new();
        let mut containers = Vec::new();
        for node in &diagram.nodes {
            let outline = Rect { left: node.x, top: node.y, right: node.x + node.width, bottom: node.y + node.height };
            if node.has_children {
                // Its name and compartments; what it holds is drawn below them.
                let header = BOX_HEADER_HEIGHT + compartments_height(&node.compartments);
                taken.push(Rect { bottom: node.y + header, ..outline });
                containers.push(outline);
            } else {
                taken.push(outline);
            }
            port_shapes(node, &mut taken);
        }
        let mut stretches = Vec::new();
        for (index, edge) in diagram.edges.iter().enumerate() {
            stretches.extend(edge.points.windows(2).map(|pair| (index, pair[0], pair[1])));
            if let Some(label) = &edge.label {
                taken.push(match &edge.label_box {
                    Some(b) => Rect { left: b.x, top: b.y, right: b.x + b.width, bottom: b.y + b.height },
                    // As the renderer writes one ELK placed nowhere: above the middle.
                    None => {
                        let (x, y) = middle(&edge.points);
                        let half = measure_text(label, 9.0, Weight::Regular) / 2.0 + 2.0;
                        Rect::around(x, y - 8.0, half, 6.0)
                    }
                });
            }
        }
        let bounds = Rect { left: 0.0, top: 0.0, right: diagram.width, bottom: diagram.height };
        Drawn { taken, containers, stretches, bounds }
    }

    /// Whether a name and its arrowhead, on line `edge`, touch nothing drawn.
    /// An arrowhead may sit where other lines run along the same stretch, as lines
    /// from one port share a trunk, but not where one crosses it.
    fn free(&self, label: Rect, arrow: Rect, edge: usize, at: Point, way: Point) -> bool {
        let label_clear = label.grown(CLEAR);
        let arrow_clear = arrow.grown(CLEAR);
        self.bounds.contains(&label_clear)
            && !self.taken.iter().any(|t| t.overlaps(&label_clear) || t.overlaps(&arrow_clear))
            && self.containers.iter().all(|c| c.contains(&label_clear) || !c.overlaps(&label_clear))
            && !self.stretches.iter().any(|(line, a, b)| {
                label_clear.crossed_by(*a, *b)
                    || (*line != edge && !along(*a, *b, at, way) && arrow_clear.crossed_by(*a, *b))
            })
    }

    /// Whether another line runs along the same stretch through `at`.
    fn shared(&self, edge: usize, at: Point, way: Point) -> bool {
        self.stretches.iter().any(|(line, a, b)| *line != edge && along(*a, *b, at, way))
    }
}

/// Whether the stretch from `a` to `b` runs the same way as `way` through `at`.
fn along(a: Point, b: Point, at: Point, way: Point) -> bool {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = dx.hypot(dy);
    if length == 0.0 || (dx * way.y - dy * way.x).abs() > 0.01 * length {
        return false;
    }
    // On the same line, and reaching `at`.
    let off_line = ((at.x - a.x) * dy - (at.y - a.y) * dx).abs() / length;
    let reach = ((at.x - a.x) * dx + (at.y - a.y) * dy) / (length * length);
    off_line < 0.5 && (-0.01..=1.01).contains(&reach)
}

/// A box's ports and the names beside them, where the renderer draws them.
fn port_shapes(node: &LaidOutNode, taken: &mut Vec<Rect>) {
    for port in &node.ports {
        let (px, py) = (node.x + port.x, node.y + port.y);
        taken.push(Rect { left: px, top: py, right: px + PORT_SIZE, bottom: py + PORT_SIZE });
        let width = measure_text(&port.label, PORT_LABEL_FONT, Weight::Regular) + 2.0;
        let (left, top) = match port.label_at {
            Some((x, y)) => (px + x, py + y),
            None => {
                let rightwards = port.x + PORT_SIZE + 6.0 + width <= node.width;
                (if rightwards { px + PORT_SIZE + 4.0 } else { px - 4.0 - width }, py - 1.0)
            }
        };
        taken.push(Rect { left, top, right: left + width, bottom: top + PORT_LABEL_FONT + 3.0 });
    }
}

fn middle(points: &[Point]) -> (f64, f64) {
    let (a, b) = (points[points.len() / 2 - 1], points[points.len() / 2]);
    ((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
}

/// The places to try for a name `width` wide on a line: an arrowhead on a stretch
/// long enough to carry one clear of its ends, the longest stretch first, and the
/// name on either side of it. Each comes with the arrowhead's centre and the way
/// the line runs there.
fn places(points: &[Point], width: f64) -> Vec<(Rect, Rect, Point, Point)> {
    let mut stretches: Vec<(Point, Point, f64)> = points
        .windows(2)
        .map(|pair| (pair[0], pair[1], (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y)))
        .collect();
    stretches.retain(|(_, _, length)| *length >= FLOW_ARROW * 3.0);
    stretches.sort_by(|a, b| b.2.total_cmp(&a.2));
    let mut found = Vec::new();
    for (from, to, length) in stretches {
        let way = Point { x: (to.x - from.x) / length, y: (to.y - from.y) / length };
        let margin = FLOW_ARROW / length;
        for fraction in FRACTIONS.iter().map(|f| f.clamp(margin, 1.0 - margin)) {
            let at = Point { x: from.x + (to.x - from.x) * fraction, y: from.y + (to.y - from.y) * fraction };
            let arrow = Rect::around(at.x, at.y, FLOW_ARROW / 2.0, FLOW_ARROW / 2.0);
            let (half_width, half_height) = (width / 2.0, LABEL_HEIGHT / 2.0);
            let beside = if way.x.abs() < 0.5 {
                // Up or down: to the left of the line, or to its right.
                [
                    Rect::around(at.x - OFFSET - half_width, at.y, half_width, half_height),
                    Rect::around(at.x + OFFSET + half_width, at.y, half_width, half_height),
                ]
            } else {
                // Sideways: above the line, or below it.
                [
                    Rect::around(at.x, at.y - OFFSET - half_height, half_width, half_height),
                    Rect::around(at.x, at.y + OFFSET + half_height, half_width, half_height),
                ]
            };
            found.extend(beside.into_iter().map(|label| (label, arrow, at, way)));
        }
    }
    found
}

/// Places the name and arrowhead of every flow on a connection. Returns false,
/// leaving the rest unplaced, as soon as one finds no free place.
pub fn place_flow_labels(diagram: &mut LaidOutDiagram) -> bool {
    let mut drawn = Drawn::of(diagram);
    for edge in 0..diagram.edges.len() {
        for flow in 0..diagram.edges[edge].flows.len() {
            let points = &diagram.edges[edge].points;
            let label = &diagram.edges[edge].flows[flow].label;
            let width = measure_text(label, FLOW_LABEL_FONT, Weight::Regular) + 2.0;
            // Where the line runs alone first, so the arrowhead is plainly its own.
            let mut candidates = places(points, width);
            candidates.sort_by_key(|(_, _, at, way)| drawn.shared(edge, *at, *way));
            let Some((label, arrow, at, way)) =
                candidates.into_iter().find(|(label, arrow, at, way)| drawn.free(*label, *arrow, edge, *at, *way))
            else {
                return false;
            };
            drawn.taken.push(label);
            // Another flow's arrowhead keeps an arrowhead's length away from this one.
            drawn.taken.push(arrow.grown(FLOW_ARROW / 2.0));
            let placed = &mut diagram.edges[edge].flows[flow];
            placed.label_box = Some(label.label_box());
            placed.arrow = Some((at, way));
        }
    }
    true
}
