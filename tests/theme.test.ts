import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import type { Model } from '../src/generated/ast.js';
import { extractUseCaseGraph } from '../src/diagrams/use-case.js';
import { layoutGraph } from '../src/layout/elk-layout.js';
import { renderSvg } from '../src/render/svg-renderer.js';
import { darkGlassTheme, getTheme, lightTheme } from '../src/render/theme.js';

const services = createSysmlServices();
const parse = parseHelper<Model>(services.Sysml);
const title = { heading: 'Use case view — DroneUseCases', source: 'fixtures/use-case.sysml' };

/** The use case view: a boundary, ellipses, an actor and «include» labels. */
async function render(theme = lightTheme): Promise<string> {
    const text = await readFile(resolve(dirname(fileURLToPath(import.meta.url)), 'fixtures/use-case.sysml'), 'utf-8');
    const document = await parse(text, { validation: true });
    const laidOut = await layoutGraph(
        extractUseCaseGraph(document.parseResult.value), { direction: 'RIGHT', edgeRouting: 'STRAIGHT' }
    );
    return renderSvg(laidOut, title, theme);
}

describe('themes', () => {
    it('paints the light theme on paper, inside a frame', async () => {
        const svg = await render();
        expect(svg).toContain('fill="white"');
        expect(svg).toContain(title.heading);
        expect(svg).toContain(title.source);
        expect(svg).toContain('fill="#fdfdf6"');
        expect(svg).not.toContain('url(#glass)');
    });

    it('paints dark glass on nothing at all, for a slide to show through', async () => {
        const svg = await render(darkGlassTheme);

        // No page: the slide is the background.
        expect(svg).not.toMatch(/<rect x="0" y="0"[^>]*fill="white"/);
        expect(svg).not.toContain('fill="white"');
        // No frame and no corner text: the slide has its own title. The
        // <title> element keeps both, so the file still names its source.
        expect(svg).not.toMatch(/<path d="M8,8 L8,30/);
        expect(svg).not.toMatch(new RegExp(`<text[^>]*>${title.source}`));
        expect(svg).toContain(`<title>${title.heading} (generated from ${title.source})</title>`);

        // Glass, lifted, with a gloss over the top.
        expect(svg).toContain('<linearGradient id="glass"');
        expect(svg).toContain('fill="url(#glass)"');
        expect(svg).toContain('fill="url(#gloss)"');
        expect(svg).toContain('filter="url(#lift)"');
        // A line round every box, and one accent colour on the edge labels.
        expect(svg).toContain(`stroke="${darkGlassTheme.boxStroke}"`);
        expect(svg).toContain(`fill="${darkGlassTheme.edgeLabel}"`);
        // The actor keeps a filled head, so the figure reads on a slide.
        expect(svg).toMatch(new RegExp(`<circle[^>]*fill="${darkGlassTheme.actorFill}"`));
    });

    it('moves nothing: both themes lay the diagram out identically', async () => {
        const positions = (svg: string) => svg.match(/(?:x|y|cx|cy|points)="[^"]*"/g);
        // Frameless light, so the frame's own coordinates are not in the way.
        const light = positions(await render({ ...lightTheme, frame: false }));
        const dark = positions(await render(darkGlassTheme));
        // The dark theme adds the gloss rectangles, so compare what the light
        // theme drew: every one of its coordinates must appear in the dark.
        for (const coordinate of new Set(light)) {
            expect(dark, coordinate).toContain(coordinate);
        }
    });

    it('knows its themes by name', () => {
        expect(getTheme('light')).toBe(lightTheme);
        expect(getTheme('dark-glass')).toBe(darkGlassTheme);
        expect(getTheme('midnight')).toBeUndefined();
    });
});
