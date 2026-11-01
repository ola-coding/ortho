/**
 * How a diagram is painted. The light theme is for documents: flat, one tint
 * on white. `dark-glass` is for slides: translucent boxes on a transparent
 * background, so whatever dark colour the slide uses shows through.
 *
 * Only colours and a few flags live here. Every position, size and line on a
 * diagram is the same in both themes, so a change of theme never moves
 * anything.
 */
export interface Theme {
    name: ThemeName;
    /** Page background, or null to leave the page transparent. */
    page: string | null;
    /**
     * A box's name compartment, which is the whole box when it holds nothing
     * else; null to tint it no differently from the rest.
     */
    nameFill: string | null;
    /** The rest of a box. */
    fill: string;
    /** The rest of a box that holds other boxes. */
    containerFill: string;
    /** Every line between boxes, and the arrowheads on them. */
    stroke: string;
    /** The outline round a box. */
    boxStroke: string;
    /** Names. */
    text: string;
    /** Keywords and compartment titles. */
    keyword: string;
    /** «include», an interface on a wire. */
    edgeLabel: string;
    /** The source path in the frame's corner. */
    provenanceText: string;
    /**
     * Painted behind a label so a line cannot strike through it, and inside
     * a message's hollow arrowhead.
     */
    halo: string;
    /** A port on a box's border. */
    portFill: string;
    /** The actor's head, filled so the figure reads at a distance. */
    actorFill: string;
    /** A highlight across the top of a box, which is what reads as glass. */
    gloss: boolean;
    /** A soft shadow under a box, lifting it off the page. */
    shadow: boolean;
    /**
     * What the diagram frame holds the drawing on, or null for no frame. Its
     * outline, the tab naming the view and the corner naming the source file
     * are painted as a box's outline and name are.
     */
    frameFill: string | null;
    /** Gradients and filters the theme's fills refer to. */
    defs: string;
}

export type ThemeName = 'light' | 'dark-glass';

/** Documents, and what the examples commit: a flat tint on white, inside a pale frame. */
export const lightTheme: Theme = {
    name: 'light',
    page: 'white',
    nameFill: '#ccdce5',
    fill: 'white',
    containerFill: 'white',
    stroke: '#1a1a1a',
    boxStroke: '#40657e',
    text: '#1a1a1a',
    keyword: '#40657e',
    edgeLabel: '#40657e',
    provenanceText: '#1a1a1a',
    halo: '#f9fbfc',
    portFill: 'white',
    actorFill: '#ccdce5',
    gloss: false,
    shadow: false,
    frameFill: '#f9fbfc',
    defs: ''
};

/**
 * Slides: no page, glass boxes lifted off whatever is behind them, and one
 * accent so a diagram is not wholly grey. The frame goes with the page — a
 * slide has its own title, and the frame's tab would only repeat it.
 */
export const darkGlassTheme: Theme = {
    name: 'dark-glass',
    page: null,
    nameFill: null,
    fill: 'url(#glass)',
    containerFill: 'url(#glassDeep)',
    stroke: '#A9C2D1',
    boxStroke: '#C3D5E0',
    text: '#E8EEF2',
    keyword: '#9FB3BF',
    edgeLabel: '#6FC7D6',
    provenanceText: '#6E828E',
    halo: '#10161C',
    portFill: '#1B242C',
    actorFill: '#C3D5E0',
    gloss: true,
    shadow: true,
    frameFill: null,
    defs: `
    <linearGradient id="glass" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#FFFFFF" stop-opacity="0.17" />
      <stop offset="1" stop-color="#FFFFFF" stop-opacity="0.06" />
    </linearGradient>
    <linearGradient id="glassDeep" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#FFFFFF" stop-opacity="0.085" />
      <stop offset="1" stop-color="#FFFFFF" stop-opacity="0.03" />
    </linearGradient>
    <linearGradient id="gloss" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#FFFFFF" stop-opacity="0.16" />
      <stop offset="1" stop-color="#FFFFFF" stop-opacity="0" />
    </linearGradient>
    <filter id="lift" x="-30%" y="-30%" width="160%" height="160%">
      <feDropShadow dx="0" dy="2" stdDeviation="3.5" flood-color="#000000" flood-opacity="0.45" />
    </filter>`
};

export const themes: Record<ThemeName, Theme> = {
    'light': lightTheme,
    'dark-glass': darkGlassTheme
};

export function getTheme(name: string): Theme | undefined {
    return themes[name as ThemeName];
}

/** `rx`/`ry` for a box, empty for square corners. */
export function radiusAttrs(radius: number): string {
    return radius > 0 ? ` rx="${radius}" ry="${radius}"` : '';
}

/** The shadow attribute for a box, empty when the theme has none. */
export function liftAttr(t: Theme): string {
    return t.shadow ? ' filter="url(#lift)"' : '';
}

/**
 * The gloss over the top of a box: a short gradient inset from its border, so
 * the light stops short of the outline rather than washing it out.
 */
export function glossOver(t: Theme, x: number, y: number, width: number, height: number, radius: number): string {
    if (!t.gloss) {
        return '';
    }
    const inset = 0.75;
    const top = Math.min(height * 0.55, 26);
    return `
      <rect x="${x + inset}" y="${y + inset}" width="${width - inset * 2}" height="${top}"${radiusAttrs(Math.max(radius - 1, 0))}
          fill="url(#gloss)" />`;
}
