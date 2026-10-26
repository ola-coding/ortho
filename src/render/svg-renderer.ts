import type { LaidOutDiagram, LaidOutEdge, LaidOutNode } from '../layout/elk-layout.js';
import { NODE_DEPTH, PORT_LABEL_FONT, PORT_SIZE } from '../layout/elk-layout.js';
import {
    BOX_HEADER_HEIGHT, COMPARTMENT_LINE_HEIGHT, COMPARTMENT_PADDING, COMPARTMENT_TITLE_HEIGHT, USAGE_RADIUS
} from '../model/graph.js';
import { measureText } from './text-metrics.js';
import type { Theme } from './theme.js';
import { glossOver, liftAttr, lightTheme, radiusAttrs } from './theme.js';

export const PAD_X = 24;
export const PAD_TOP = 48;
/** Room below the drawing, clear of the frame's corner compartment. */
const PAD_BOTTOM = 30;
export const FONT = 'Arial, Helvetica, Segoe UI, sans-serif';

/** A port is a usage too, at a port's size. */
const PORT_RADIUS = 2;

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
 * What the frame says: the view's name and definition in its tab, and the
 * model file that declares the view in its bottom-right corner
 * (everything-as-code: every SVG names the file that generated it).
 */
export interface DiagramTitle {
    /** The view's name; empty for an unnamed view. */
    heading: string;
    /** Its view definition, as written: `GeneralView`. */
    definition?: string;
    source: string;
}

/** The frame's inset from the page, and its corners: a view is a usage. */
const FRAME_INSET = 8;
const FRAME_RADIUS = 10;
const TAB_HEIGHT = 22;
const TAB_RADIUS = 8;
const INFO_HEIGHT = 15;
const INFO_RADIUS = 6;

/**
 * The view frame (§8.2.3.26): a rounded border, a tab in its top-left corner
 * reading `«view» name : Definition`, and an info compartment in its
 * bottom-right corner holding the source file.
 */
export function renderFrame(width: number, height: number, title: DiagramTitle, t: Theme): string {
    if (!t.frame) {
        return '';
    }
    const left = FRAME_INSET;
    const top = FRAME_INSET;
    const right = width - FRAME_INSET;
    const bottom = height - FRAME_INSET;
    const name = title.heading ? ` ${title.heading}` : '';
    const definition = title.definition ? ` : ${title.definition}` : '';
    const tabRight = left + 22 + measureText('«view»', 11) + measureText(name, 11, 'bold') + measureText(definition, 11);
    const tabBottom = top + TAB_HEIGHT;
    const infoLeft = right - measureText(title.source, 9) - 16;
    const infoTop = bottom - INFO_HEIGHT;
    const line = `stroke="${t.boxStroke}" stroke-width="1.25"`;
    return `
  <rect x="${left}" y="${top}" width="${right - left}" height="${bottom - top}"${radiusAttrs(FRAME_RADIUS)} fill="none" ${line} />
  <path d="M${left},${tabBottom} L${left},${top + FRAME_RADIUS} A${FRAME_RADIUS},${FRAME_RADIUS} 0 0 1 ${left + FRAME_RADIUS},${top} L${tabRight},${top} L${tabRight},${tabBottom - TAB_RADIUS} A${TAB_RADIUS},${TAB_RADIUS} 0 0 1 ${tabRight - TAB_RADIUS},${tabBottom} Z"
        fill="#f4f4ea" ${line} />
  <text x="${left + 10}" y="${top + 15}" font-size="11" fill="${t.text}"><tspan fill="${t.keyword}">&#171;view&#187;</tspan><tspan font-weight="600">${escapeXml(name)}</tspan>${escapeXml(definition)}</text>
  <path d="M${infoLeft},${bottom} L${infoLeft},${infoTop + INFO_RADIUS} A${INFO_RADIUS},${INFO_RADIUS} 0 0 1 ${infoLeft + INFO_RADIUS},${infoTop} L${right},${infoTop}"
        fill="none" ${line} />
  <text x="${right - 8}" y="${bottom - 4}" text-anchor="end" font-size="9" fill="${t.provenanceText}">${escapeXml(title.source)}</text>`;
}

/** The SVG's `<title>`: the view, and the file it was generated from. */
export function titleText(title: DiagramTitle): string {
    return `${title.heading || title.definition || ''} (generated from ${title.source})`;
}

function renderNode(n: LaidOutNode, t: Theme): string {
    switch (n.shape) {
        case 'actor': return renderActorNode(n, t);
        case 'package': return renderPackageNode(n, t);
        case 'node3d': return renderNode3dNode(n, t);
        case 'definition':
        case 'usage': return renderBoxNode(n, t);
    }
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
      <rect x="${x}" y="${y + d}" width="${w - d}" height="${h - d}"
          fill="${t.fill}" stroke="${t.boxStroke}" stroke-width="1.25" />${glossOver(t, x, y + d, w - d, h - d, 0)}
      <text x="${x + (w - d) / 2}" y="${y + d + 20}" text-anchor="middle"
          font-size="12" font-weight="600" fill="${t.text}">${escapeXml(n.name)}</text></g>`;
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
      <rect x="${x}" y="${y + 12}" width="${n.width}" height="${n.height - 12}"
          fill="${t.fill}" stroke="${t.boxStroke}" stroke-width="1.25" />${glossOver(t, x, y + 12, n.width, n.height - 12, 0)}
      ${name}</g>`;
}

/**
 * A definition or a usage: a box with square or rounded corners, its keyword
 * above its name, compartments below and ports on its border.
 */
function renderBoxNode(n: LaidOutNode, t: Theme): string {
    const x = n.x + PAD_X;
    const y = n.y + PAD_TOP;
    const radius = n.shape === 'usage' ? USAGE_RADIUS : 0;
    const parts: string[] = [];

    // A box that contains other boxes (an assembly on the Physical view) takes
    // a slightly deeper tint, so nesting reads before any border is traced.
    parts.push(`<rect x="${x}" y="${y}" width="${n.width}" height="${n.height}"${radiusAttrs(radius)}${liftAttr(t)}
          fill="${n.hasChildren ? t.containerFill : t.fill}" stroke="${t.boxStroke}" stroke-width="1.25" />`);
    const gloss = glossOver(t, x, y, n.width, n.height, radius).trim();
    if (gloss) {
        parts.push(gloss);
    }

    // The keyword and name are centred in the header, which is the whole box
    // when it holds nothing else.
    const header = n.compartments.length > 0 || n.hasChildren ? HEADER_HEIGHT : n.height;
    const top = y + (header - HEADER_HEIGHT) / 2;
    if (n.keyword) {
        parts.push(`<text x="${x + n.width / 2}" y="${top + 13}" text-anchor="middle"
          font-size="9" fill="${t.keyword}">&#171;${escapeXml(n.keyword)}&#187;</text>`);
        parts.push(`<text x="${x + n.width / 2}" y="${top + 26}" text-anchor="middle"
          font-size="12" font-weight="600" fill="${t.text}">${escapeXml(n.name)}</text>`);
    } else {
        parts.push(`<text x="${x + n.width / 2}" y="${top + 21}" text-anchor="middle"
          font-size="12" font-weight="600" fill="${t.text}">${escapeXml(n.name)}</text>`);
    }

    let compartmentTop = y + HEADER_HEIGHT;
    for (const compartment of n.compartments) {
        parts.push(`<line x1="${x}" y1="${compartmentTop}" x2="${x + n.width}" y2="${compartmentTop}"
          stroke="${t.boxStroke}" stroke-width="1" />`);
        let linesTop = compartmentTop;
        if (compartment.title) {
            parts.push(`<text x="${x + 8}" y="${compartmentTop + 11}"
          font-size="9" fill="${t.keyword}">${escapeXml(compartment.title)}</text>`);
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
        parts.push(`<rect x="${x + port.x}" y="${y + port.y}" width="${PORT_SIZE}" height="${PORT_SIZE}"${radiusAttrs(PORT_RADIUS)}
          fill="${t.portFill}" stroke="${t.stroke}" stroke-width="1.25" />`);
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

function renderEdge(e: LaidOutEdge, t: Theme): string {
    if (e.points.length < 2) {
        return '';
    }
    const points = e.points;
    const pointsAttr = points.map(p => `${p.x + PAD_X},${p.y + PAD_TOP}`).join(' ');
    // «include» and «import» carry an open arrowhead and their keyword; an
    // «import» is dashed (§8.2.3.5), an «include» solid (§8.2.3.25). Every
    // other line is plain: a connection, a use case's line, or a branch of
    // the function tree, whose direction the layout carries.
    const keyworded = e.kind === 'include' || e.kind === 'import';
    const dash = e.kind === 'import' ? ` stroke-dasharray="6 4"` : '';
    const markers = keyworded ? ` marker-end="url(#openArrow)"` : '';
    let svg = `\n    <polyline points="${pointsAttr}" fill="none" stroke="${t.stroke}" stroke-width="1.25"${dash}${markers} />`;
    if (e.label) {
        // A keyword gets guillemets; a name, such as a connection's
        // interface, does not.
        const label = keyworded ? `&#171;${escapeXml(e.label)}&#187;` : escapeXml(e.label);
        // A label ELK placed is centred in the box ELK reserved for it. Any
        // other sits just above the line's midpoint.
        const box = e.labelBox;
        const midpoint = pointAtFraction(points, 0.5);
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

    const nodeElements = diagram.nodes.map(n => renderNode(n, t)).join('');
    const edgeElements = diagram.edges.map(e => renderEdge(e, t)).join('');
    const portLabelElements = diagram.nodes.map(n => renderPortLabels(n, t)).join('');

    return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"
     viewBox="0 0 ${width} ${height}" font-family="${FONT}">
  <title>${escapeXml(titleText(title))}</title>
  <defs>${t.defs}
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
