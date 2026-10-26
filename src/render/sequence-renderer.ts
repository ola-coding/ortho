import type { SequenceModel } from '../diagrams/sequence.js';
import { USAGE_RADIUS } from '../model/graph.js';
import { escapeXml, renderFrame, titleText, FONT, PAD_X, PAD_TOP } from './svg-renderer.js';
import type { DiagramTitle } from './svg-renderer.js';
import { measureText } from './text-metrics.js';
import type { Theme } from './theme.js';
import { glossOver, liftAttr, lightTheme, radiusAttrs } from './theme.js';


/** A lifeline's head: a part, «part» above its name (§8.2.3.9). */
const HEAD_HEIGHT = 36;
const MESSAGE_STEP = 40;
const SELF_LOOP_WIDTH = 40;

/**
 * Sequence layout is deterministic (lifelines across the top, messages in
 * order down the page), so this renderer does its own layout instead of ELK.
 * A message runs from lifeline to lifeline (§8.2.3.9): SysML v2 draws no
 * activation bars.
 */
export function renderSequenceSvg(model: SequenceModel, title: DiagramTitle, t: Theme = lightTheme): string {
    const heads = model.lifelines.map(l => ({ ...l, width: Math.max(90, measureText(l.label, 12, 'bold') + 24) }));
    const columnOf = new Map(heads.map((h, i) => [h.id, i]));

    // Per-gap spacing: start from head widths, then widen the gaps each
    // message label spans until the label fits.
    const gaps: number[] = [];
    for (let i = 0; i + 1 < heads.length; i++) {
        gaps.push(Math.max(120, heads[i].width / 2 + heads[i + 1].width / 2 + 30));
    }
    let rightExtra = 0;
    for (const message of model.messages) {
        const a = columnOf.get(message.sourceId)!;
        const b = columnOf.get(message.targetId)!;
        const labelWidth = measureText(message.label, 10);
        if (a === b) {
            const needed = SELF_LOOP_WIDTH + labelWidth + 24;
            if (a === heads.length - 1) {
                rightExtra = Math.max(rightExtra, needed - heads[a].width / 2);
            } else {
                gaps[a] = Math.max(gaps[a], needed + heads[a + 1].width / 2);
            }
        } else {
            const [lo, hi] = a < b ? [a, b] : [b, a];
            const span = gaps.slice(lo, hi).reduce((sum, g) => sum + g, 0);
            const needed = labelWidth + 50;
            if (span < needed) {
                const grow = (needed - span) / (hi - lo);
                for (let g = lo; g < hi; g++) {
                    gaps[g] += grow;
                }
            }
        }
    }

    const centers = new Map<string, number>();
    let x = PAD_X + (heads[0]?.width ?? 90) / 2;
    heads.forEach((h, i) => {
        centers.set(h.id, x);
        x += gaps[i] ?? 0;
    });
    const lastHead = heads[heads.length - 1];
    const contentRight = lastHead ? centers.get(lastHead.id)! + lastHead.width / 2 + rightExtra : 200;

    const headTop = PAD_TOP;
    const firstMessageY = headTop + HEAD_HEIGHT + 36;
    const lifelineBottom = firstMessageY + model.messages.length * MESSAGE_STEP;
    const width = contentRight + PAD_X;
    // Lifelines end clear of the frame's corner compartment.
    const height = lifelineBottom + 34;

    const parts: string[] = [];

    for (const head of heads) {
        const cx = centers.get(head.id)!;
        parts.push(`
    <g><rect x="${cx - head.width / 2}" y="${headTop}" width="${head.width}" height="${HEAD_HEIGHT}"${radiusAttrs(USAGE_RADIUS)}${liftAttr(t)}
          fill="${t.fill}" stroke="${t.boxStroke}" stroke-width="1.25" />${glossOver(t, cx - head.width / 2, headTop, head.width, HEAD_HEIGHT, USAGE_RADIUS)}
      <text x="${cx}" y="${headTop + 15}" text-anchor="middle"
          font-size="9" fill="${t.keyword}">&#171;part&#187;</text>
      <text x="${cx}" y="${headTop + 28}" text-anchor="middle"
          font-size="12" font-weight="600" fill="${t.text}">${escapeXml(head.label)}</text>
      <line x1="${cx}" y1="${headTop + HEAD_HEIGHT}" x2="${cx}" y2="${lifelineBottom}"
          stroke="${t.stroke}" stroke-width="1" stroke-dasharray="4 4" /></g>`);
    }

    model.messages.forEach((message, i) => {
        const y = firstMessageY + i * MESSAGE_STEP;
        const sourceX = centers.get(message.sourceId)!;
        const targetX = centers.get(message.targetId)!;
        const label = escapeXml(message.label);
        if (message.sourceId === message.targetId) {
            const right = sourceX + SELF_LOOP_WIDTH;
            parts.push(`
    <g><polyline points="${sourceX},${y} ${right},${y} ${right},${y + 16} ${sourceX},${y + 16}"
          fill="none" stroke="${t.stroke}" stroke-width="1.25" marker-end="url(#msgArrow)" />
      <text x="${right + 6}" y="${y + 12}" font-size="10" fill="${t.text}">${label}</text></g>`);
        } else {
            parts.push(`
    <g><line x1="${sourceX}" y1="${y}" x2="${targetX}" y2="${y}"
          stroke="${t.stroke}" stroke-width="1.25" marker-end="url(#msgArrow)" />
      <text x="${(sourceX + targetX) / 2}" y="${y - 6}" text-anchor="middle" font-size="10"
          fill="${t.text}" paint-order="stroke" stroke="${t.halo}" stroke-width="3">${label}</text></g>`);
        }
    });

    return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"
     viewBox="0 0 ${width} ${height}" font-family="${FONT}">
  <title>${escapeXml(titleText(title))}</title>
  <defs>${t.defs}
    <marker id="msgArrow" markerWidth="13" markerHeight="11" refX="11" refY="5" orient="auto"
            markerUnits="userSpaceOnUse">
      <path d="M1,1 L11,5 L1,9 z" fill="${t.halo}" stroke="${t.stroke}" stroke-width="1.25" />
    </marker>
  </defs>
  ${t.page ? `<rect x="0" y="0" width="${width}" height="${height}" fill="${t.page}" />` : ''}${renderFrame(width, height, title, t)}
  ${parts.join('')}
</svg>
`;
}
