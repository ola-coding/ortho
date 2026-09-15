import type { SequenceModel } from '../diagrams/sequence.js';
import { escapeXml, renderFrame, FONT, PAD_X, PAD_TOP } from './svg-renderer.js';
import type { DiagramTitle } from './svg-renderer.js';
import { measureText } from './text-metrics.js';

const FILL = '#fdfdf6';
const STROKE = '#2b2b2b';
const TEXT_COLOR = '#1a1a1a';

const HEAD_HEIGHT = 34;
/** A stick figure and the name beneath it. */
const ACTOR_HEIGHT = 74;
const MESSAGE_STEP = 40;
const SELF_LOOP_WIDTH = 44;
const BAR_WIDTH = 8;

interface Activation {
    lifelineId: string;
    fromY: number;
    toY: number;
}

/**
 * Sequence layout is deterministic (lifelines across the top, messages in
 * order down the page), so this renderer does its own layout instead of ELK.
 */
export function renderSequenceSvg(model: SequenceModel, title: DiagramTitle): string {
    const heads = model.lifelines.map(l => ({
        ...l,
        width: l.actor
            ? Math.max(60, measureText(l.label, 12, 'bold') + 16)
            : Math.max(90, measureText(l.label, 12, 'bold') + 24)
    }));
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
    // A stick figure is taller than a box. When one is present every head
    // takes its height, boxes bottom-aligned, so all lifelines start level.
    const headsHeight = heads.some(h => h.actor) ? ACTOR_HEIGHT : HEAD_HEIGHT;
    const firstMessageY = headTop + headsHeight + 36;
    const lifelineBottom = firstMessageY + model.messages.length * MESSAGE_STEP;
    const width = contentRight + PAD_X;
    const height = lifelineBottom + 28;

    // Activation bars: a lifeline is active from a message's arrival until the
    // next message it sends (heuristic; good enough until fragments exist).
    const activations: Activation[] = [];
    model.messages.forEach((message, i) => {
        const y = firstMessageY + i * MESSAGE_STEP;
        if (message.sourceId === message.targetId) {
            activations.push({ lifelineId: message.targetId, fromY: y, toY: y + 24 });
            return;
        }
        const next = model.messages.findIndex((m, j) => j > i && m.sourceId === message.targetId);
        const toY = next >= 0 ? firstMessageY + next * MESSAGE_STEP : y + 24;
        activations.push({ lifelineId: message.targetId, fromY: y, toY });
    });

    const parts: string[] = [];

    for (const head of heads) {
        const cx = centers.get(head.id)!;
        if (head.actor) {
            // The same figure the use case view draws, so a person reads as
            // one on both views.
            const top = headTop;
            parts.push(`
    <g><g stroke="${STROKE}" stroke-width="1.5" fill="none">
        <circle cx="${cx}" cy="${top + 9}" r="8" fill="${FILL}" />
        <line x1="${cx}" y1="${top + 17}" x2="${cx}" y2="${top + 36}" />
        <line x1="${cx - 13}" y1="${top + 24}" x2="${cx + 13}" y2="${top + 24}" />
        <line x1="${cx}" y1="${top + 36}" x2="${cx - 11}" y2="${top + 52}" />
        <line x1="${cx}" y1="${top + 36}" x2="${cx + 11}" y2="${top + 52}" /></g>
      <text x="${cx}" y="${top + 68}" text-anchor="middle"
          font-size="12" font-weight="600" fill="${TEXT_COLOR}">${escapeXml(head.label)}</text>
      <line x1="${cx}" y1="${headTop + headsHeight}" x2="${cx}" y2="${lifelineBottom}"
          stroke="${STROKE}" stroke-width="1" stroke-dasharray="4 4" /></g>`);
            continue;
        }
        const boxTop = headTop + headsHeight - HEAD_HEIGHT;
        parts.push(`
    <g><rect x="${cx - head.width / 2}" y="${boxTop}" width="${head.width}" height="${HEAD_HEIGHT}"
          fill="${FILL}" stroke="${STROKE}" stroke-width="1.25" />
      <text x="${cx}" y="${boxTop + HEAD_HEIGHT / 2 + 4}" text-anchor="middle"
          font-size="12" font-weight="600" fill="${TEXT_COLOR}">${escapeXml(head.label)}</text>
      <line x1="${cx}" y1="${boxTop + HEAD_HEIGHT}" x2="${cx}" y2="${lifelineBottom}"
          stroke="${STROKE}" stroke-width="1" stroke-dasharray="4 4" /></g>`);
    }

    for (const activation of activations) {
        const cx = centers.get(activation.lifelineId)!;
        parts.push(`
    <rect x="${cx - BAR_WIDTH / 2}" y="${activation.fromY}" width="${BAR_WIDTH}" height="${activation.toY - activation.fromY}"
          fill="#f1f1e4" stroke="${STROKE}" stroke-width="1" />`);
    }

    model.messages.forEach((message, i) => {
        const y = firstMessageY + i * MESSAGE_STEP;
        const sourceX = centers.get(message.sourceId)!;
        const targetX = centers.get(message.targetId)!;
        const label = escapeXml(message.label);
        if (message.sourceId === message.targetId) {
            const right = sourceX + BAR_WIDTH / 2 + SELF_LOOP_WIDTH - BAR_WIDTH;
            parts.push(`
    <g><polyline points="${sourceX + BAR_WIDTH / 2},${y} ${right},${y} ${right},${y + 16} ${sourceX + BAR_WIDTH / 2 + 4},${y + 16}"
          fill="none" stroke="${STROKE}" stroke-width="1.25" marker-end="url(#msgArrow)" />
      <text x="${right + 6}" y="${y + 12}" font-size="10" fill="${TEXT_COLOR}">${label}</text></g>`);
        } else {
            // Arrows meet the edge of the activation bar, not the lifeline.
            const direction = Math.sign(targetX - sourceX);
            const x1 = sourceX + direction * (BAR_WIDTH / 2);
            const x2 = targetX - direction * (BAR_WIDTH / 2);
            parts.push(`
    <g><line x1="${x1}" y1="${y}" x2="${x2}" y2="${y}"
          stroke="${STROKE}" stroke-width="1.25" marker-end="url(#msgArrow)" />
      <text x="${(x1 + x2) / 2}" y="${y - 6}" text-anchor="middle" font-size="10"
          fill="${TEXT_COLOR}" paint-order="stroke" stroke="white" stroke-width="3">${label}</text></g>`);
        }
    });

    return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"
     viewBox="0 0 ${width} ${height}" font-family="${FONT}">
  <title>${escapeXml(`${title.heading} (generated from ${title.source})`)}</title>
  <defs>
    <marker id="msgArrow" markerWidth="12" markerHeight="10" refX="10" refY="4" orient="auto"
            markerUnits="userSpaceOnUse">
      <path d="M0,0 L10,4 L0,8 z" fill="${STROKE}" />
    </marker>
  </defs>
  <rect x="0" y="0" width="${width}" height="${height}" fill="white" />${renderFrame(width, height, title)}
  ${parts.join('')}
</svg>
`;
}
