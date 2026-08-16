/**
 * Deterministic text measurement without a browser or font files: standard
 * Arial/Helvetica advance widths (1/1000 em) for ASCII 32–126. The SVG font
 * stack leads with Arial so these metrics match what viewers render; Segoe UI
 * (fallback) is slightly narrower, so sizes err on the safe side.
 */

// prettier-ignore
const REGULAR = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278,
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556,
    1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778,
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556,
    333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556,
    556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584
];

// prettier-ignore
const BOLD = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278,
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611,
    975, 722, 722, 722, 722, 667, 611, 778, 722, 278, 556, 722, 611, 833, 722, 778,
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 333, 278, 333, 584, 556,
    333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556, 278, 889, 611, 611,
    611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584
];

export type FontWeight = 'regular' | 'bold';

export function measureText(text: string, fontSize: number, weight: FontWeight = 'regular'): number {
    const table = weight === 'bold' ? BOLD : REGULAR;
    let units = 0;
    for (const ch of text) {
        const code = ch.codePointAt(0)!;
        if (code >= 32 && code <= 126) {
            units += table[code - 32];
        } else if (code === 0xab || code === 0xbb) {
            units += 556; // « »
        } else {
            units += 600;
        }
    }
    return (units * fontSize) / 1000;
}
