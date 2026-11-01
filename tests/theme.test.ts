import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import type { Model } from '../src/generated/ast.js';
import { extractUseCaseGraph } from '../src/diagrams/use-case.js';
import { layoutUseCases } from '../src/layout/use-case-layout.js';
import { renderFrame, renderSvg } from '../src/render/svg-renderer.js';
import { darkGlassTheme, getTheme, lightTheme } from '../src/render/theme.js';

const services = createSysmlServices();
const parse = parseHelper<Model>(services.Sysml);
const title = { heading: 'Use case view — DroneUseCases', definition: 'GeneralView', source: 'fixtures/use-case.sysml' };

/** The use case view: «use case» boxes, a «subject», an actor and «include» labels. */
async function render(theme = lightTheme): Promise<string> {
    const text = await readFile(resolve(dirname(fileURLToPath(import.meta.url)), 'fixtures/use-case.sysml'), 'utf-8');
    const document = await parse(text, { validation: true });
    const laidOut = layoutUseCases(extractUseCaseGraph(document.parseResult.value.packages));
    return renderSvg(laidOut, title, theme);
}

describe('themes', () => {
    it('paints the light theme on paper, inside a frame', async () => {
        const svg = await render();
        expect(svg).toContain('fill="white"');
        // The frame's tab names the view (§8.2.3.26); its definition is
        // optional there, and left out.
        expect(svg).toContain(`&#171;view&#187;</tspan><tspan font-weight="600"> ${title.heading}</tspan></text>`);
        expect(svg).not.toContain(': GeneralView');
        expect(svg).toContain(title.source);
        expect(svg).toContain(`fill="${lightTheme.nameFill}"`);
        expect(svg).not.toContain('url(#glass)');
    });

    it('tints a box up to the end of its name compartment, and leaves the rest white', () => {
        const box = {
            id: 'a', shape: 'usage' as const, keyword: 'part', name: 'a', ports: [], x: 0, y: 0, width: 200, height: 90
        };
        const svgOf = (node: object, theme = lightTheme) => renderSvg(
            { width: 200, height: 90, edges: [], nodes: [{ ...box, compartments: [], hasChildren: false, ...node }] },
            title, theme
        );
        const tinted = `fill="${lightTheme.nameFill}"`;
        const band = /<path d="[^"]*" fill="#ccdce5"/;

        // Nothing else inside: the whole box is its name compartment.
        expect(svgOf({})).toMatch(/<rect[^>]*height="90"[^>]*fill="#ccdce5"/);
        expect(svgOf({})).not.toMatch(band);

        // With compartments or nested boxes, the band ends where the name does.
        for (const node of [{ compartments: [{ lines: ['x'] }] }, { hasChildren: true }]) {
            expect(svgOf(node)).toMatch(band);
            expect(svgOf(node)).toMatch(/<rect[^>]*height="90"[^>]*fill="white"/);
        }

        // A theme without a tint of its own draws no band.
        const dark = svgOf({ hasChildren: true }, darkGlassTheme);
        expect(dark).not.toContain(tinted);
        expect(dark.replace(/<defs>[\s\S]*<\/defs>/, '')).not.toContain('<path');
    });

    it('paints the frame as the boxes inside it are painted', () => {
        const frame = renderFrame(400, 300, title, lightTheme);
        expect(frame).toContain(`fill="${lightTheme.frameFill}"`);
        // The tab and the corner take a box's tint, all three a box's outline.
        expect(frame.match(new RegExp(`fill="${lightTheme.nameFill}"`, 'g'))).toHaveLength(2);
        expect(frame.match(new RegExp(`stroke="${lightTheme.boxStroke}"`, 'g'))).toHaveLength(3);
        expect(renderFrame(400, 300, title, darkGlassTheme)).toBe('');
    });

    it('paints dark glass on nothing at all, for a slide to show through', async () => {
        const svg = await render(darkGlassTheme);

        // No page: the slide is the background.
        expect(svg).not.toMatch(/<rect x="0" y="0"[^>]*fill="white"/);
        expect(svg).not.toContain('fill="white"');
        // No frame and no corner text: the slide has its own title. The
        // <title> element keeps both, so the file still names its source.
        expect(svg).not.toContain('&#171;view&#187;');
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
        const light = positions(await render({ ...lightTheme, frameFill: null }));
        const dark = positions(await render(darkGlassTheme));
        // The dark theme adds the gloss rectangles, so compare what the light
        // theme drew: every one of its coordinates must appear in the dark.
        for (const coordinate of new Set(light)) {
            expect(dark, coordinate).toContain(coordinate);
        }
    });

    it('names an unnamed view by its definition', () => {
        const frame = renderFrame(400, 300, { heading: '', definition: 'GeneralView', source: 'inline' }, lightTheme);
        expect(frame).toContain('&#171;view&#187;</tspan><tspan font-weight="600"></tspan> : GeneralView</text>');
    });

    it('knows its themes by name', () => {
        expect(getTheme('light')).toBe(lightTheme);
        expect(getTheme('dark-glass')).toBe(darkGlassTheme);
        expect(getTheme('midnight')).toBeUndefined();
    });
});
