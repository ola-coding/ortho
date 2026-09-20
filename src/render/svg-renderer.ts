import type { LaidOutDiagram, LaidOutEdge, LaidOutNode } from '../layout/elk-layout.js';
import { NODE_DEPTH, PORT_LABEL_FONT, PORT_SIZE } from '../layout/elk-layout.js';
import {
    BOX_HEADER_HEIGHT, COMPARTMENT_LINE_HEIGHT, COMPARTMENT_PADDING, COMPARTMENT_TITLE_HEIGHT
} from '../model/graph.js';
import { measureText } from './text-metrics.js';
import type { Theme } from './theme.js';
import { glossOver, liftAttr, lightTheme, radiusAttrs } from './theme.js';

export const PAD_X = 24;
export const PAD_TOP = 48;
const PAD_BOTTOM = 24;
export const FONT = 'Arial, Helvetica, Segoe UI, sans-serif';

// Must match the extractor's sizing constants.
const HEADER_HEIGHT = BOX_HEADER_HEIGHT;
const LINE_HEIGHT = COMPARTMENT_LINE_HEIGHT;

export function escapeXml(text: string): string {
    return text
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;');
}

/**
 * Diagram identity: the view heading shown in the frame pentagon, and the
 * source model file shown as provenance in the frame's bottom-right corner
 * (everything-as-code: every SVG names the file that generated it).
 */
export interface DiagramTitle {
    heading: string;
    source: string;
}

/** Diagram frame: border, pentagon heading top-left, provenance bottom-right. */
export function renderFrame(width: number, height: number, title: DiagramTitle, t: Theme): string {
    if (!t.frame) {
        return '';
    }
    const headingWidth = measureText(title.heading, 11, 'bold') + 28;
    return `
  <rect x="8" y="8" width="${width - 16}" height="${height - 16}" fill="none" stroke="${t.boxStroke}" stroke-width="1.25" />
  <path d="M8,8 L8,30 L${8 + headingWidth - 10},30 L${8 + headingWidth},21 L${8 + headingWidth},8 Z"
        fill="#f4f4ea" stroke="${t.boxStroke}" stroke-width="1.25" />
  <text x="16" y="23" font-size="11" font-weight="600" fill="${t.text}">${escapeXml(title.heading)}</text>
  <text x="${width - 14}" y="${height - 14}" text-anchor="end" font-size="9" fill="${t.provenanceText}">${escapeXml(title.source)}</text>`;
}

function renderNode(n: LaidOutNode, t: Theme): string {
    switch (n.shape) {
        case 'ellipse': return renderEllipseNode(n, t);
        case 'actor': return renderActorNode(n, t);
        case 'boundary': return renderBoundaryNode(n, t);
        case 'package': return renderPackageNode(n, t);
        case 'rounded': return renderRoundedNode(n, t);
        case 'node3d': return renderNode3dNode(n, t);
        case 'box': return renderBoxNode(n, t);
    }
}

/** Logical view: one function, drawn as a leaf of the capability tree. */
function renderRoundedNode(n: LaidOutNode, t: Theme): string {
    const x = n.x + PAD_X;
    const y = n.y + PAD_TOP;
    return `
    <g><rect x="${x}" y="${y}" width="${n.width}" height="${n.height}" rx="11" ry="11"${liftAttr(t)}
          fill="${t.fill}" stroke="${t.boxStroke}" stroke-width="1.25" />${glossOver(t, x, y, n.width, n.height)}
      <text x="${x + n.width / 2}" y="${y + n.height / 2 + 4}" text-anchor="middle"
          font-size="12" fill="${t.text}">${escapeXml(n.name)}</text></g>`;
}

/**
 * Deployment view: a hardware host. The depth edge is drawn *inside* the node's
 * box, so the front face is inset by NODE_DEPTH at the top and right and the
 * whole solid still fits the bounds ELK allocated.
 */
function renderNode3dNode(n: LaidOutNode, t: Theme): string {
    const x = n.x + PAD_X;
    const y = n.y + PAD_TOP;
    const w = n.width;
    const h = n.height;
    const d = NODE_DEPTH;
    return `
    <g><polygon points="${x},${y + d} ${x + d},${y} ${x + w},${y} ${x + w - d},${y + d}"
          fill="${t.node3dFill}" stroke="${t.boxStroke}" stroke-width="1.25" />
      <polygon points="${x + w - d},${y + d} ${x + w},${y} ${x + w},${y + h - d} ${x + w - d},${y + h}"
          fill="${t.node3dFill}" stroke="${t.boxStroke}" stroke-width="1.25" />
      <rect x="${x}" y="${y + d}" width="${w - d}" height="${h - d}"${radiusAttrs(t)}
          fill="${t.fill}" stroke="${t.boxStroke}" stroke-width="1.25" />${glossOver(t, x, y + d, w - d, h - d)}
      <text x="${x + (w - d) / 2}" y="${y + d + 20}" text-anchor="middle"
          font-size="12" font-weight="600" fill="${t.text}">${escapeXml(n.name)}</text></g>`;
}

function renderEllipseNode(n: LaidOutNode, t: Theme): string {
    const cx = n.x + PAD_X + n.width / 2;
    const cy = n.y + PAD_TOP + n.height / 2;
    return `
    <g><ellipse cx="${cx}" cy="${cy}" rx="${n.width / 2}" ry="${n.height / 2}"${liftAttr(t)}
          fill="${t.fill}" stroke="${t.boxStroke}" stroke-width="1.25" />
      <text x="${cx}" y="${cy + 4}" text-anchor="middle"
          font-size="12" fill="${t.text}">${escapeXml(n.name)}</text></g>`;
}

function renderActorNode(n: LaidOutNode, t: Theme): string {
    const cx = n.x + PAD_X + n.width / 2;
    const top = n.y + PAD_TOP;
    return `
    <g stroke="${t.stroke}" stroke-width="1.5" fill="none">
      <circle cx="${cx}" cy="${top + 9}" r="8" fill="${t.actorFill}" />
      <line x1="${cx}" y1="${top + 17}" x2="${cx}" y2="${top + 36}" />
      <line x1="${cx - 13}" y1="${top + 24}" x2="${cx + 13}" y2="${top + 24}" />
      <line x1="${cx}" y1="${top + 36}" x2="${cx - 11}" y2="${top + 52}" />
      <line x1="${cx}" y1="${top + 36}" x2="${cx + 11}" y2="${top + 52}" />
      <text x="${cx}" y="${top + 68}" text-anchor="middle" stroke="none"
          font-size="11" fill="${t.text}">${escapeXml(n.name)}</text>
    </g>`;
}

function renderBoundaryNode(n: LaidOutNode, t: Theme): string {
    const x = n.x + PAD_X;
    const y = n.y + PAD_TOP;
    return `
    <g><rect x="${x}" y="${y}" width="${n.width}" height="${n.height}"${radiusAttrs(t)}
          fill="${t.boundaryFill}" stroke="${t.boxStroke}" stroke-width="1.25" />${glossOver(t, x, y, n.width, n.height)}
      <text x="${x + n.width / 2}" y="${y + 20}" text-anchor="middle"
          font-size="12" font-weight="600" fill="${t.text}">${escapeXml(n.name)}</text></g>`;
}

function renderPackageNode(n: LaidOutNode, t: Theme): string {
    const x = n.x + PAD_X;
    const y = n.y + PAD_TOP;
    const tabWidth = Math.max(46, Math.min(90, measureText(n.name, 11, 'bold') + 20));
    // EA convention: container packages carry their name in the tab area,
    // leaf packages center it in the body.
    const name = n.hasChildren
        ? `<text x="${x + 10}" y="${y + 27}" font-size="11" font-weight="600" fill="${t.text}">${escapeXml(n.name)}</text>`
        : `<text x="${x + n.width / 2}" y="${y + 12 + (n.height - 12) / 2 + 4}" text-anchor="middle"
          font-size="12" font-weight="600" fill="${t.text}">${escapeXml(n.name)}</text>`;
    return `
    <g><rect x="${x}" y="${y}" width="${tabWidth}" height="12"
          fill="${t.fill}" stroke="${t.boxStroke}" stroke-width="1.25" />
      <rect x="${x}" y="${y + 12}" width="${n.width}" height="${n.height - 12}"${radiusAttrs(t)}
          fill="${t.fill}" stroke="${t.boxStroke}" stroke-width="1.25" />${glossOver(t, x, y + 12, n.width, n.height - 12)}
      ${name}</g>`;
}

function renderBoxNode(n: LaidOutNode, t: Theme): string {
    const x = n.x + PAD_X;
    const y = n.y + PAD_TOP;
    const parts: string[] = [];

    // A box that contains other boxes (an assembly on the Physical view) takes
    // a slightly deeper tint, so nesting reads before any border is traced.
    parts.push(`<rect x="${x}" y="${y}" width="${n.width}" height="${n.height}"${radiusAttrs(t)}${liftAttr(t)}
          fill="${n.hasChildren ? t.containerFill : t.fill}" stroke="${t.boxStroke}" stroke-width="1.25" />`);
    const gloss = glossOver(t, x, y, n.width, n.height).trim();
    if (gloss) {
        parts.push(gloss);
    }

    // An unstereotyped box (an Implementation-view module, a deployed software
    // part) centres its name in the header instead of leaving an empty «».
    if (n.stereotype) {
        parts.push(`<text x="${x + n.width / 2}" y="${y + 13}" text-anchor="middle"
          font-size="9" fill="${t.stereotype}">&#171;${escapeXml(n.stereotype)}&#187;</text>`);
        parts.push(`<text x="${x + n.width / 2}" y="${y + 26}" text-anchor="middle"
          font-size="12" font-weight="600" fill="${t.text}">${escapeXml(n.name)}</text>`);
    } else {
        parts.push(`<text x="${x + n.width / 2}" y="${y + 21}" text-anchor="middle"
          font-size="12" font-weight="600" fill="${t.text}">${escapeXml(n.name)}</text>`);
    }

    let compartmentTop = y + HEADER_HEIGHT;
    for (const compartment of n.compartments) {
        parts.push(`<line x1="${x}" y1="${compartmentTop}" x2="${x + n.width}" y2="${compartmentTop}"
          stroke="${t.boxStroke}" stroke-width="1" />`);
        let linesTop = compartmentTop;
        if (compartment.title) {
            parts.push(`<text x="${x + 8}" y="${compartmentTop + 11}"
          font-size="9" fill="${t.stereotype}">${escapeXml(compartment.title)}</text>`);
            linesTop += COMPARTMENT_TITLE_HEIGHT;
        }
        compartment.lines.forEach((line, i) => {
            parts.push(`<text x="${x + 8}" y="${linesTop + 12 + i * LINE_HEIGHT}"
          font-size="11" fill="${t.text}">${escapeXml(line)}</text>`);
        });
        compartmentTop += COMPARTMENT_PADDING + (compartment.title ? COMPARTMENT_TITLE_HEIGHT : 0)
            + compartment.lines.length * LINE_HEIGHT;
    }

    for (const port of n.ports) {
        parts.push(`<rect x="${x + port.x}" y="${y + port.y}" width="${PORT_SIZE}" height="${PORT_SIZE}"
          fill="${t.markerFill}" stroke="${t.stroke}" stroke-width="1.25" />`);
    }

    return `\n    <g>${parts.join('\n      ')}</g>`;
}

/**
 * Port labels are painted after every edge, so a line passing a port never
 * strikes through its name. ELK places them (outside the box, beside the
 * port, on whichever side avoids a neighbour); the fallback only covers a
 * port ELK returned no label position for.
 */
function renderPortLabels(n: LaidOutNode, t: Theme): string {
    const x = n.x + PAD_X;
    const y = n.y + PAD_TOP;
    return n.ports.map(port => {
        const px = x + port.x;
        const py = y + port.y;
        const halo = `font-size="${PORT_LABEL_FONT}" fill="${t.text}" paint-order="stroke" stroke="${t.halo}" stroke-width="2.5"`;
        if (port.labelX !== undefined && port.labelY !== undefined) {
            return `\n    <text x="${px + port.labelX}" y="${py + port.labelY + PORT_LABEL_FONT}" ${halo}>${escapeXml(port.label)}</text>`;
        }
        const labelWidth = measureText(port.label, PORT_LABEL_FONT);
        const rightwards = port.x + PORT_SIZE + 6 + labelWidth <= n.width;
        const labelX = rightwards ? px + PORT_SIZE + 4 : px - 4;
        return `\n    <text x="${labelX}" y="${py + PORT_SIZE / 2 + 3}"${rightwards ? '' : ' text-anchor="end"'} ${halo}>${escapeXml(port.label)}</text>`;
    }).join('');
}

function pointAtFraction(points: Array<{ x: number; y: number }>, fraction: number): { x: number; y: number } {
    let total = 0;
    const lengths: number[] = [];
    for (let i = 1; i < points.length; i++) {
        const len = Math.hypot(points[i].x - points[i - 1].x, points[i].y - points[i - 1].y);
        lengths.push(len);
        total += len;
    }
    let remaining = total * fraction;
    for (let i = 1; i < points.length; i++) {
        if (remaining <= lengths[i - 1]) {
            const t = lengths[i - 1] === 0 ? 0 : remaining / lengths[i - 1];
            return {
                x: points[i - 1].x + (points[i].x - points[i - 1].x) * t,
                y: points[i - 1].y + (points[i].y - points[i - 1].y) * t
            };
        }
        remaining -= lengths[i - 1];
    }
    return points[points.length - 1];
}

/**
 * ELK attaches edges to the node's bounding box; for ellipse nodes, extend the
 * terminal segment until it meets the ellipse curve.
 */
function clipEndpointToEllipse(points: Array<{ x: number; y: number }>, node: LaidOutNode, atStart: boolean): void {
    if (points.length < 2) {
        return;
    }
    const index = atStart ? 0 : points.length - 1;
    const neighbour = atStart ? 1 : points.length - 2;
    const p = points[index];
    const q = points[neighbour];
    const rx = node.width / 2;
    const ry = node.height / 2;
    const cx = node.x + rx;
    const cy = node.y + ry;
    const dx = p.x - q.x;
    const dy = p.y - q.y;
    if (dx === 0 && dy === 0) {
        return;
    }
    const ex = (p.x - cx) / rx;
    const ey = (p.y - cy) / ry;
    const fx = dx / rx;
    const fy = dy / ry;
    const a = fx * fx + fy * fy;
    const b = 2 * (ex * fx + ey * fy);
    const c = ex * ex + ey * ey - 1;
    // Already on (or numerically inside) the curve — e.g. a bbox endpoint at
    // the ellipse's axis extremes — must not clip to the far intersection.
    if (c <= 1e-3) {
        return;
    }
    const disc = b * b - 4 * a * c;
    if (a === 0 || disc < 0) {
        return;
    }
    const sqrt = Math.sqrt(disc);
    const t = [(-b - sqrt) / (2 * a), (-b + sqrt) / (2 * a)].filter(v => v >= 0).sort((u, v) => u - v)[0];
    // Bbox-to-curve adjustments are small; larger jumps mean the geometry is
    // not the expected case, so leave the point alone.
    if (t === undefined || t * Math.hypot(dx, dy) > Math.max(rx, ry)) {
        return;
    }
    points[index] = { x: p.x + dx * t, y: p.y + dy * t };
}

function renderEdge(e: LaidOutEdge, nodeById: Map<string, LaidOutNode>, t: Theme): string {
    if (e.points.length < 2) {
        return '';
    }
    const points = e.points.map(p => ({ ...p }));
    const sourceNode = nodeById.get(e.sourceId);
    const targetNode = nodeById.get(e.targetId);
    if (sourceNode?.shape === 'ellipse') {
        clipEndpointToEllipse(points, sourceNode, true);
    }
    if (targetNode?.shape === 'ellipse') {
        clipEndpointToEllipse(points, targetNode, false);
    }

    const pointsAttr = points.map(p => `${p.x + PAD_X},${p.y + PAD_TOP}`).join(' ');
    let markers = '';
    let dash = '';
    switch (e.kind) {
        case 'specialization':
            // Layout edges run super → sub, so the triangle sits at the start.
            markers = ` marker-start="url(#triangle)"`;
            break;
        case 'composition':
            markers = ` marker-start="url(#diamond)"`;
            break;
        case 'satisfy':
        case 'include':
        case 'import':
        case 'allocate':
        case 'typing':
            markers = ` marker-end="url(#openArrow)"`;
            dash = ` stroke-dasharray="6 4"`;
            break;
        case 'connection':
        case 'association':
        // Capability-tree branches: a plain line, no arrowhead. Direction is
        // carried by the layout (parents above children), not by a marker.
        case 'decomposition':
            break;
    }
    let svg = `\n    <polyline points="${pointsAttr}" fill="none" stroke="${t.stroke}" stroke-width="1.25"${dash}${markers} />`;
    if (e.label) {
        // Relationship-keyword labels render as stereotypes; a user-given name
        // (e.g. a named allocation) renders plainly, like a role name.
        const stereotyped = ['satisfy', 'include', 'import', 'typing'].includes(e.kind)
            || (e.kind === 'allocate' && e.label === 'allocate');
        const label = stereotyped ? `&#171;${escapeXml(e.label)}&#187;` : escapeXml(e.label);
        // A label ELK placed is centred in the box ELK reserved for it. Any
        // other sits just above the line's midpoint — or, for a composition,
        // near the part end, as EA places role names.
        const box = e.labelBox;
        const midpoint = pointAtFraction(points, e.kind === 'composition' ? 0.82 : 0.5);
        const textX = (box ? box.x + box.width / 2 : midpoint.x) + PAD_X;
        const textY = box ? box.y + box.height / 2 + 3 + PAD_TOP : midpoint.y + PAD_TOP - 5;
        svg += `\n    <text x="${textX}" y="${textY}" text-anchor="middle" font-size="9"
          fill="${t.edgeLabel}" paint-order="stroke" stroke="${t.halo}" stroke-width="3">${label}</text>`;
    }
    return svg;
}

export function renderSvg(diagram: LaidOutDiagram, title: DiagramTitle, t: Theme = lightTheme): string {
    const width = diagram.width + PAD_X * 2;
    const height = diagram.height + PAD_TOP + PAD_BOTTOM;
    const nodeById = new Map(diagram.nodes.map(n => [n.id, n]));

    const nodeElements = diagram.nodes.map(n => renderNode(n, t)).join('');
    const edgeElements = diagram.edges.map(e => renderEdge(e, nodeById, t)).join('');
    const portLabelElements = diagram.nodes.map(n => renderPortLabels(n, t)).join('');

    return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"
     viewBox="0 0 ${width} ${height}" font-family="${FONT}">
  <title>${escapeXml(`${title.heading} (generated from ${title.source})`)}</title>
  <defs>${t.defs}
    <marker id="triangle" markerWidth="14" markerHeight="12" refX="12" refY="5"
            orient="auto-start-reverse" markerUnits="userSpaceOnUse">
      <path d="M0,0 L12,5 L0,10 z" fill="${t.markerFill}" stroke="${t.boxStroke}" stroke-width="1.25" />
    </marker>
    <marker id="diamond" markerWidth="18" markerHeight="12" refX="15" refY="5"
            orient="auto-start-reverse" markerUnits="userSpaceOnUse">
      <path d="M1,5 L8,1 L15,5 L8,9 z" fill="${t.stroke}" stroke="${t.boxStroke}" />
    </marker>
    <marker id="openArrow" markerWidth="12" markerHeight="12" refX="10" refY="5"
            orient="auto" markerUnits="userSpaceOnUse">
      <path d="M0,0 L10,5 L0,10" fill="none" stroke="${t.boxStroke}" stroke-width="1.25" />
    </marker>
  </defs>
  ${t.page ? `<rect x="0" y="0" width="${width}" height="${height}" fill="${t.page}" />` : ''}${renderFrame(width, height, title, t)}
  ${nodeElements}
  ${edgeElements}${portLabelElements ? `\n  ${portLabelElements}` : ''}
</svg>
`;
}
