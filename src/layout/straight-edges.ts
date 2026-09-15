import type { LaidOutDiagram, LaidOutEdge, LaidOutNode } from './elk-layout.js';
import { measureText } from '../render/text-metrics.js';

/**
 * Straight lines for the use case view. ELK places the shapes and leaves room
 * for each «include» label; this pass then redraws every line it can as a
 * single straight segment, the way use case diagrams are drawn by hand, and
 * sets each label beside its line.
 *
 * A line ends on an ellipse's outline, or beside an actor's figure. Aimed at
 * the ellipse's centre it may clip a neighbouring use case, so its ends slide
 * — round the outline, and along the actor's side — until the line is clear.
 * Where a use case stands right behind another as an actor sees it, no
 * sliding helps; the actor is then nudged up or down its column until it can
 * see past, preferring a place where its lines cross no other. A line that
 * nothing clears keeps ELK's route, bent round what is in the way.
 */

interface Point {
    x: number;
    y: number;
}

interface Rect {
    x: number;
    y: number;
    width: number;
    height: number;
}

type Line = [Point, Point];

/** Half the span of the stick figure's arms (drawn by svg-renderer.ts). */
const FIGURE_HALF_WIDTH = 13;
/** A line to an actor stops this far short of its hands. */
const ACTOR_GAP = 6;
/** The band down the side of an actor's figure, below the top of its box, where lines meet it. */
const BAND_TOP = 10;
const BAND_BOTTOM = 48;
/** Lines meeting one side of an actor are ideally this far apart, and never closer than MIN_ANCHOR_GAP. */
const ANCHOR_SPACING = 8;
const MIN_ANCHOR_GAP = 5;
/** A line's end moves along an actor's side in steps of this much. */
const ANCHOR_STEP = 2;
/** Clearance a line keeps from every shape it does not end at. */
const CLEARANCE = 4;
/** An end slides round an ellipse in steps of this angle, up to MAX_STEPS either way. */
const STEP = Math.PI / 24;
const MAX_STEPS = 10;
/**
 * The most obliquely a line may meet an ellipse: the least cosine allowed
 * between the line and the outline's normal where they meet.
 */
const MIN_INCIDENCE = 0.2;
/** An actor is nudged in steps of NUDGE_STEP, up to MAX_NUDGE either way. */
const NUDGE_STEP = 4;
const MAX_NUDGE = 60;
/** Room a nudged actor keeps from every other shape. */
const ACTOR_ROOM = 10;
/** ELK's padding round the whole drawing. */
const DRAWING_PADDING = 12;
/** Gap between a label and its line. */
const LABEL_GAP = 3;
/** Label font and height, as ELK sized them (elk-layout.ts). */
const LABEL_FONT = 9;
const LABEL_HEIGHT = 12;

/** One candidate end of a line; `normal` is the outline's, for an end on an ellipse. */
interface EndPoint {
    point: Point;
    normal?: Point;
    steps: number;
}

type Obstacle = (p: Point, q: Point) => boolean;

const centre = (n: Rect): Point => ({ x: n.x + n.width / 2, y: n.y + n.height / 2 });

function unit(v: Point): Point {
    const length = Math.hypot(v.x, v.y);
    return length === 0 ? { x: 0, y: 0 } : { x: v.x / length, y: v.y / length };
}

/** Whether segment pq passes within an ellipse of radii rx, ry about c. */
function crossesEllipse(p: Point, q: Point, c: Point, rx: number, ry: number): boolean {
    // Scaled so the ellipse is the unit circle: the nearest point of the
    // segment to the centre then says it all.
    const ax = (p.x - c.x) / rx;
    const ay = (p.y - c.y) / ry;
    const dx = (q.x - p.x) / rx;
    const dy = (q.y - p.y) / ry;
    const lengthSq = dx * dx + dy * dy;
    const t = lengthSq === 0 ? 0 : Math.max(0, Math.min(1, -(ax * dx + ay * dy) / lengthSq));
    const nx = ax + t * dx;
    const ny = ay + t * dy;
    return nx * nx + ny * ny < 1;
}

/** Whether segment pq passes through a rectangle (Liang–Barsky). */
function crossesRect(p: Point, q: Point, r: Rect): boolean {
    const dx = q.x - p.x;
    const dy = q.y - p.y;
    let t0 = 0;
    let t1 = 1;
    for (const [towards, room] of [
        [-dx, p.x - r.x], [dx, r.x + r.width - p.x],
        [-dy, p.y - r.y], [dy, r.y + r.height - p.y]
    ]) {
        if (towards === 0) {
            if (room < 0) {
                return false;
            }
            continue;
        }
        const t = room / towards;
        if (towards < 0) {
            t0 = Math.max(t0, t);
        } else {
            t1 = Math.min(t1, t);
        }
        if (t0 > t1) {
            return false;
        }
    }
    return true;
}

/** Whether two lines cross at a point inside both; lines that only touch do not. */
function crosses([a, b]: Line, [c, d]: Line): boolean {
    const side = (p: Point, q: Point, r: Point): number =>
        Math.sign((q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x));
    return side(a, b, c) * side(a, b, d) < 0 && side(c, d, a) * side(c, d, b) < 0;
}

const inflate = (r: Rect, by: number): Rect =>
    ({ x: r.x - by, y: r.y - by, width: r.width + 2 * by, height: r.height + 2 * by });

const overlaps = (a: Rect, b: Rect): boolean =>
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;

/** The strip a boundary's title is drawn in (svg-renderer.ts). */
function titleStrip(n: LaidOutNode): Rect {
    const width = measureText(n.name, 12, 'bold') + 8;
    return { x: n.x + (n.width - width) / 2, y: n.y + 6, width, height: 20 };
}

/**
 * The places a line may end on an ellipse: the point facing `toward` first,
 * then points further round the outline, alternately either side of it.
 */
function outlinePoints(n: LaidOutNode, toward: Point): EndPoint[] {
    const rx = n.width / 2;
    const ry = n.height / 2;
    const c = centre(n);
    const facing = Math.atan2((toward.y - c.y) / ry, (toward.x - c.x) / rx);
    const points: EndPoint[] = [];
    for (let steps = 0; steps <= MAX_STEPS; steps++) {
        for (const sign of steps === 0 ? [1] : [1, -1]) {
            const angle = facing + sign * steps * STEP;
            points.push({
                point: { x: c.x + rx * Math.cos(angle), y: c.y + ry * Math.sin(angle) },
                normal: unit({ x: Math.cos(angle) / rx, y: Math.sin(angle) / ry }),
                steps
            });
        }
    }
    return points;
}

/** An end on an ellipse must face the other end, not meet it edge-on. */
function faces(end: EndPoint, other: Point): boolean {
    if (!end.normal) {
        return true;
    }
    const towards = unit({ x: other.x - end.point.x, y: other.y - end.point.y });
    return towards.x * end.normal.x + towards.y * end.normal.y >= MIN_INCIDENCE;
}

/** The least slid pair of ends whose line is clear of every obstacle, if any. */
function clearLine(fromEnds: EndPoint[], toEnds: EndPoint[], obstacles: Obstacle[]): Line | undefined {
    const pairs = fromEnds.flatMap(from => toEnds.map(to => ({ from, to })));
    // The sort is stable, so ties go the same way on every run.
    pairs.sort((a, b) => (a.from.steps + a.to.steps) - (b.from.steps + b.to.steps));
    const clear = pairs.find(({ from, to }) => faces(from, to.point) && faces(to, from.point)
        && obstacles.every(blocks => !blocks(from.point, to.point)));
    return clear && [clear.from.point, clear.to.point];
}

/** Values from `min` to `max`, ANCHOR_STEP apart, nearest to `ideal` first. */
function around(ideal: number, min: number, max: number): number[] {
    const start = Math.max(min, Math.min(max, ideal));
    const values: number[] = [];
    for (let d = 0; start + d <= max || start - d >= min; d += ANCHOR_STEP) {
        if (start + d <= max) {
            values.push(start + d);
        }
        if (d > 0 && start - d >= min) {
            values.push(start - d);
        }
    }
    return values;
}

/**
 * Straight lines from an actor to each of its use cases — or undefined if one
 * cannot be drawn, unless `partial`, which leaves that one out instead. The
 * lines meet the actor beside its figure, on the side facing each use case,
 * in the order they leave it, so they never cross each other there; each is
 * as near its even share of that side as a clear line allows.
 */
function actorLines(
    actor: LaidOutNode,
    own: Array<{ edge: LaidOutEdge; other: LaidOutNode }>,
    obstaclesFor: (a: LaidOutNode, b: LaidOutNode) => Obstacle[],
    partial: boolean
): Map<string, Line> | undefined {
    const spine = actor.x + actor.width / 2;
    const middle = actor.y + (BAND_TOP + BAND_BOTTOM) / 2;
    const lines = new Map<string, Line>();
    for (const side of [-1, 1]) {
        const facing = own
            .filter(({ other }) => (centre(other).x < spine ? -1 : 1) === side)
            .map(line => ({
                ...line,
                angle: Math.atan2(centre(line.other).y - middle, Math.abs(centre(line.other).x - spine))
            }))
            .sort((a, b) => a.angle - b.angle);
        const count = facing.length;
        const spacing = count > 1 ? Math.min(ANCHOR_SPACING, (BAND_BOTTOM - BAND_TOP) / (count - 1)) : 0;
        const x = spine + side * (FIGURE_HALF_WIDTH + ACTOR_GAP);
        let highest = actor.y + BAND_TOP;
        for (const [i, { edge, other }] of facing.entries()) {
            const ideal = middle + (i - (count - 1) / 2) * spacing;
            // Leave room below for the lines still to come.
            const lowest = actor.y + BAND_BOTTOM - (count - 1 - i) * MIN_ANCHOR_GAP;
            let line: Line | undefined;
            for (const y of around(ideal, highest, lowest)) {
                const anchor = { x, y };
                line = clearLine([{ point: anchor, steps: 0 }], outlinePoints(other, anchor), obstaclesFor(actor, other));
                if (line) {
                    break;
                }
            }
            if (!line) {
                if (!partial) {
                    return undefined;
                }
                continue;
            }
            lines.set(edge.id, edge.sourceId === actor.id ? line : [line[1], line[0]]);
            highest = line[0].y + MIN_ANCHOR_GAP;
        }
    }
    return lines;
}

/**
 * A label beside the middle of its line: above or below a shallow line, to
 * the right or left of a steep one — whichever first keeps clear of every
 * shape and every other line.
 */
function labelBeside(p: Point, q: Point, width: number, busy: Rect[], lines: Line[]): Rect {
    const middle = { x: (p.x + q.x) / 2, y: (p.y + q.y) / 2 };
    const dx = Math.abs(q.x - p.x);
    const dy = Math.abs(q.y - p.y);
    // How far to lift the label so its lower corners clear the line, or to
    // shift it so its inner corners do.
    const rise = dx === 0 ? Infinity : LABEL_HEIGHT / 2 + LABEL_GAP + (dy / dx) * width / 2;
    const shift = dy === 0 ? Infinity : width / 2 + LABEL_GAP + (dx / dy) * LABEL_HEIGHT / 2;
    const at = (x: number, y: number): Rect =>
        ({ x: x - width / 2, y: y - LABEL_HEIGHT / 2, width, height: LABEL_HEIGHT });
    const vertical = [at(middle.x, middle.y - rise), at(middle.x, middle.y + rise)];
    const horizontal = [at(middle.x + shift, middle.y), at(middle.x - shift, middle.y)];
    const candidates = (rise <= shift ? [...vertical, ...horizontal] : [...horizontal, ...vertical])
        .filter(box => Number.isFinite(box.x) && Number.isFinite(box.y));
    return candidates.find(box => busy.every(b => !overlaps(box, b))
        && lines.every(([a, b]) => !crossesRect(a, b, box))) ?? candidates[0];
}

const isEnd = (n?: LaidOutNode): n is LaidOutNode => n?.shape === 'ellipse' || n?.shape === 'actor';

export function straightenEdges(diagram: LaidOutDiagram): LaidOutDiagram {
    // Actors may be nudged, so work on copies.
    const nodes = diagram.nodes.map(n => ({ ...n }));
    const nodeById = new Map(nodes.map(n => [n.id, n]));
    const shapes = nodes.filter(isEnd);
    const titles = nodes.filter(n => n.shape === 'boundary').map(titleStrip);
    const obstaclesFor = (a: LaidOutNode, b: LaidOutNode): Obstacle[] => [
        ...shapes.filter(n => n !== a && n !== b).map((n): Obstacle => n.shape === 'ellipse'
            ? (p, q) => crossesEllipse(p, q, centre(n), n.width / 2 + CLEARANCE, n.height / 2 + CLEARANCE)
            : (p, q) => crossesRect(p, q, inflate(n, CLEARANCE))),
        ...titles.map((strip): Obstacle => (p, q) => crossesRect(p, q, strip))
    ];
    const lines = new Map<string, Line>();

    // Lines between use cases first: they do not depend on where any actor
    // stands, and each actor's lines are then placed to cross none of them.
    for (const edge of diagram.edges) {
        const source = nodeById.get(edge.sourceId);
        const target = nodeById.get(edge.targetId);
        if (source?.shape === 'ellipse' && target?.shape === 'ellipse') {
            const line = clearLine(
                outlinePoints(source, centre(target)), outlinePoints(target, centre(source)),
                obstaclesFor(source, target)
            );
            if (line) {
                lines.set(edge.id, line);
            }
        }
    }

    // ELK centres an actor on its use cases, which can leave one use case
    // standing right behind another as the actor sees it. Nudge the actor up
    // or down its column, as little as will do, until every one of its lines
    // can be straight — going further only to find a place where none of
    // them crosses a line already drawn. An actor no nudge helps stays where
    // ELK put it, with what lines can be straight.
    const offsets = [0];
    for (let d = NUDGE_STEP; d <= MAX_NUDGE; d += NUDGE_STEP) {
        offsets.push(d, -d);
    }
    for (const actor of nodes.filter(n => n.shape === 'actor')) {
        const own = diagram.edges.flatMap(edge => {
            const otherId = edge.sourceId === actor.id ? edge.targetId
                : edge.targetId === actor.id ? edge.sourceId : undefined;
            const other = otherId === undefined ? undefined : nodeById.get(otherId);
            return other?.shape === 'ellipse' ? [{ edge, other }] : [];
        });
        const drawn = [...lines.values()];
        const home = actor.y;
        let best: { offset: number; crossings: number; lines: Map<string, Line> } | undefined;
        for (const offset of offsets) {
            actor.y = home + offset;
            if (actor.y < Math.min(home, DRAWING_PADDING)
                || nodes.some(n => n !== actor && overlaps(inflate(actor, ACTOR_ROOM), n))) {
                continue;
            }
            const placed = actorLines(actor, own, obstaclesFor, false);
            if (!placed) {
                continue;
            }
            const crossings = [...placed.values()]
                .reduce((sum, line) => sum + drawn.filter(other => crosses(line, other)).length, 0);
            if (!best || crossings < best.crossings) {
                best = { offset, crossings, lines: placed };
            }
            if (crossings === 0) {
                break;
            }
        }
        actor.y = home + (best?.offset ?? 0);
        for (const [id, line] of best?.lines ?? actorLines(actor, own, obstaclesFor, true)!) {
            lines.set(id, line);
        }
    }

    const edges = diagram.edges.map(edge => {
        const line = lines.get(edge.id);
        return line ? { ...edge, points: [...line] } : edge;
    });

    // Labels go last, so each can keep clear of every line. A label on a line
    // left to ELK keeps ELK's place.
    const busy = [...shapes.map(n => inflate(n, 2)), ...titles];
    const segments = (except: LaidOutEdge): Line[] => edges
        .filter(e => e !== except)
        .flatMap(e => e.points.slice(1).map((p, i): Line => [e.points[i], p]));
    return {
        width: diagram.width,
        // A nudged actor may reach below everything ELK placed.
        height: Math.max(diagram.height, ...nodes.map(n => n.y + n.height + DRAWING_PADDING)),
        nodes,
        edges: edges.map(edge => {
            if (!edge.label || !lines.has(edge.id)) {
                return edge;
            }
            const [p, q] = edge.points;
            const width = edge.labelBox?.width ?? measureText(`«${edge.label}»`, LABEL_FONT) + 4;
            return { ...edge, labelBox: labelBeside(p, q, width, busy, segments(edge)) };
        })
    };
}
