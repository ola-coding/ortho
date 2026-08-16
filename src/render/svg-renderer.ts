import type { LaidOutDiagram, LaidOutEdge, LaidOutNode } from '../layout/elk-layout.js';
import { PORT_SIZE } from '../layout/elk-layout.js';
import { measureText } from './text-metrics.js';

export const PAD_X = 24;
export const PAD_TOP = 48;
const PAD_BOTTOM = 24;
const FILL = '#fdfdf6';
const STROKE = '#2b2b2b';
const TEXT_COLOR = '#1a1a1a';
const STEREOTYPE_COLOR = '#555555';
export const FONT = 'Arial, Helvetica, Segoe UI, sans-serif';

// Must match the extractor's sizing constants.
const HEADER_HEIGHT = 32;
const LINE_HEIGHT = 15;
const COMPARTMENT_PADDING = 8;

export function escapeXml(text: string): string {
    return text
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;');
}

/**
 * Diagram identity: the 4+1 view heading shown in the frame pentagon, and the
 * source model file shown as provenance in the frame's bottom-right corner
 * (everything-as-code: every SVG names the file that generated it).
 */
export interface DiagramTitle {
    heading: string;
    source: string;
}

/** Diagram frame: border, pentagon heading top-left, provenance bottom-right. */
export function renderFrame(width: number, height: number, title: DiagramTitle): string {
    const headingWidth = measureText(title.heading, 11, 'bold') + 28;
    return `
  <rect x="8" y="8" width="${width - 16}" height="${height - 16}" fill="none" stroke="${STROKE}" stroke-width="1.25" />
  <path d="M8,8 L8,30 L${8 + headingWidth - 10},30 L${8 + headingWidth},21 L${8 + headingWidth},8 Z"
        fill="#f4f4ea" stroke="${STROKE}" stroke-width="1.25" />
  <text x="16" y="23" font-size="11" font-weight="600" fill="${TEXT_COLOR}">${escapeXml(title.heading)}</text>
  <text x="${width - 14}" y="${height - 14}" text-anchor="end" font-size="9" fill="#8a8a86">${escapeXml(title.source)}</text>`;
}

function renderNode(n: LaidOutNode): string {
    switch (n.shape) {
        case 'ellipse': return renderEllipseNode(n);
        case 'actor': return renderActorNode(n);
        case 'boundary': return renderBoundaryNode(n);
        case 'package': return renderPackageNode(n);
        case 'box': return renderBoxNode(n);
    }
}

function renderEllipseNode(n: LaidOutNode): string {
    const cx = n.x + PAD_X + n.width / 2;
    const cy = n.y + PAD_TOP + n.height / 2;
    return `
    <g><ellipse cx="${cx}" cy="${cy}" rx="${n.width / 2}" ry="${n.height / 2}"
          fill="${FILL}" stroke="${STROKE}" stroke-width="1.25" />
      <text x="${cx}" y="${cy + 4}" text-anchor="middle"
          font-size="12" fill="${TEXT_COLOR}">${escapeXml(n.name)}</text></g>`;
}

function renderActorNode(n: LaidOutNode): string {
    const cx = n.x + PAD_X + n.width / 2;
    const top = n.y + PAD_TOP;
    return `
    <g stroke="${STROKE}" stroke-width="1.5" fill="none">
      <circle cx="${cx}" cy="${top + 9}" r="8" fill="${FILL}" />
      <line x1="${cx}" y1="${top + 17}" x2="${cx}" y2="${top + 36}" />
      <line x1="${cx - 13}" y1="${top + 24}" x2="${cx + 13}" y2="${top + 24}" />
      <line x1="${cx}" y1="${top + 36}" x2="${cx - 11}" y2="${top + 52}" />
      <line x1="${cx}" y1="${top + 36}" x2="${cx + 11}" y2="${top + 52}" />
      <text x="${cx}" y="${top + 68}" text-anchor="middle" stroke="none"
          font-size="11" fill="${TEXT_COLOR}">${escapeXml(n.name)}</text>
    </g>`;
}

function renderBoundaryNode(n: LaidOutNode): string {
    const x = n.x + PAD_X;
    const y = n.y + PAD_TOP;
    return `
    <g><rect x="${x}" y="${y}" width="${n.width}" height="${n.height}"
          fill="#fbfbf1" stroke="${STROKE}" stroke-width="1.25" />
      <text x="${x + n.width / 2}" y="${y + 20}" text-anchor="middle"
          font-size="12" font-weight="600" fill="${TEXT_COLOR}">${escapeXml(n.name)}</text></g>`;
}

function renderPackageNode(n: LaidOutNode): string {
    const x = n.x + PAD_X;
    const y = n.y + PAD_TOP;
    const tabWidth = Math.max(46, Math.min(90, measureText(n.name, 11, 'bold') + 20));
    // EA convention: container packages carry their name in the tab area,
    // leaf packages center it in the body.
    const name = n.hasChildren
        ? `<text x="${x + 10}" y="${y + 27}" font-size="11" font-weight="600" fill="${TEXT_COLOR}">${escapeXml(n.name)}</text>`
        : `<text x="${x + n.width / 2}" y="${y + 12 + (n.height - 12) / 2 + 4}" text-anchor="middle"
          font-size="12" font-weight="600" fill="${TEXT_COLOR}">${escapeXml(n.name)}</text>`;
    return `
    <g><rect x="${x}" y="${y}" width="${tabWidth}" height="12"
          fill="${FILL}" stroke="${STROKE}" stroke-width="1.25" />
      <rect x="${x}" y="${y + 12}" width="${n.width}" height="${n.height - 12}"
          fill="${FILL}" stroke="${STROKE}" stroke-width="1.25" />
      ${name}</g>`;
}

function renderBoxNode(n: LaidOutNode): string {
    const x = n.x + PAD_X;
    const y = n.y + PAD_TOP;
    const parts: string[] = [];

    parts.push(`<rect x="${x}" y="${y}" width="${n.width}" height="${n.height}"
          fill="${FILL}" stroke="${STROKE}" stroke-width="1.25" />`);

    parts.push(`<text x="${x + n.width / 2}" y="${y + 13}" text-anchor="middle"
          font-size="9" fill="${STEREOTYPE_COLOR}">&#171;${escapeXml(n.stereotype)}&#187;</text>`);
    parts.push(`<text x="${x + n.width / 2}" y="${y + 26}" text-anchor="middle"
          font-size="12" font-weight="600" fill="${TEXT_COLOR}">${escapeXml(n.name)}</text>`);

    let compartmentTop = y + HEADER_HEIGHT;
    for (const compartment of n.compartments) {
        parts.push(`<line x1="${x}" y1="${compartmentTop}" x2="${x + n.width}" y2="${compartmentTop}"
          stroke="${STROKE}" stroke-width="1" />`);
        compartment.lines.forEach((line, i) => {
            parts.push(`<text x="${x + 8}" y="${compartmentTop + 12 + i * LINE_HEIGHT}"
          font-size="11" fill="${TEXT_COLOR}">${escapeXml(line)}</text>`);
        });
        compartmentTop += COMPARTMENT_PADDING + compartment.lines.length * LINE_HEIGHT;
    }

    for (const port of n.ports) {
        const px = x + port.x;
        const py = y + port.y;
        parts.push(`<rect x="${px}" y="${py}" width="${PORT_SIZE}" height="${PORT_SIZE}"
          fill="white" stroke="${STROKE}" stroke-width="1.25" />`);
        // Label beside the square (which straddles the border), clear of both
        // the header text and the edge leaving the port.
        const labelWidth = measureText(port.label, 9);
        const rightwards = port.x + PORT_SIZE + 6 + labelWidth <= n.width;
        const labelX = rightwards ? px + PORT_SIZE + 4 : px - 4;
        parts.push(`<text x="${labelX}" y="${py + PORT_SIZE / 2 + 3}"${rightwards ? '' : ' text-anchor="end"'}
          font-size="9" fill="${TEXT_COLOR}" paint-order="stroke" stroke="white" stroke-width="2.5">${escapeXml(port.label)}</text>`);
    }

    return `\n    <g>${parts.join('\n      ')}</g>`;
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

function renderEdge(e: LaidOutEdge, nodeById: Map<string, LaidOutNode>): string {
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
            break;
    }
    let svg = `\n    <polyline points="${pointsAttr}" fill="none" stroke="${STROKE}" stroke-width="1.25"${dash}${markers} />`;
    if (e.label) {
        // Relationship-keyword labels render as stereotypes; a user-given name
        // (e.g. a named allocation) renders plainly, like a role name.
        const stereotyped = ['satisfy', 'include', 'import', 'typing'].includes(e.kind)
            || (e.kind === 'allocate' && e.label === 'allocate');
        // EA places role names near the far (part) end of a composition.
        const anchor = pointAtFraction(points, e.kind === 'composition' ? 0.82 : 0.5);
        const label = stereotyped ? `&#171;${escapeXml(e.label)}&#187;` : escapeXml(e.label);
        svg += `\n    <text x="${anchor.x + PAD_X}" y="${anchor.y + PAD_TOP - 5}" text-anchor="middle" font-size="9"
          fill="${STEREOTYPE_COLOR}" paint-order="stroke" stroke="white" stroke-width="3">${label}</text>`;
    }
    return svg;
}

export function renderSvg(diagram: LaidOutDiagram, title: DiagramTitle): string {
    const width = diagram.width + PAD_X * 2;
    const height = diagram.height + PAD_TOP + PAD_BOTTOM;
    const nodeById = new Map(diagram.nodes.map(n => [n.id, n]));

    const nodeElements = diagram.nodes.map(renderNode).join('');
    const edgeElements = diagram.edges.map(e => renderEdge(e, nodeById)).join('');

    return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"
     viewBox="0 0 ${width} ${height}" font-family="${FONT}">
  <title>${escapeXml(`${title.heading} (generated from ${title.source})`)}</title>
  <defs>
    <marker id="triangle" markerWidth="14" markerHeight="12" refX="12" refY="5"
            orient="auto-start-reverse" markerUnits="userSpaceOnUse">
      <path d="M0,0 L12,5 L0,10 z" fill="white" stroke="${STROKE}" stroke-width="1.25" />
    </marker>
    <marker id="diamond" markerWidth="18" markerHeight="12" refX="15" refY="5"
            orient="auto-start-reverse" markerUnits="userSpaceOnUse">
      <path d="M1,5 L8,1 L15,5 L8,9 z" fill="${STROKE}" stroke="${STROKE}" />
    </marker>
    <marker id="openArrow" markerWidth="12" markerHeight="12" refX="10" refY="5"
            orient="auto" markerUnits="userSpaceOnUse">
      <path d="M0,0 L10,5 L0,10" fill="none" stroke="${STROKE}" stroke-width="1.25" />
    </marker>
  </defs>
  <rect x="0" y="0" width="${width}" height="${height}" fill="white" />${renderFrame(width, height, title)}
  ${nodeElements}
  ${edgeElements}
</svg>
`;
}
