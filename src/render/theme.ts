/**
 * How a diagram is painted. The light theme is ortho's own drawing on paper;
 * `dark-glass` is for slides: translucent boxes on a transparent background,
 * so whatever dark colour the slide uses shows through.
 *
 * Only colours and a few flags live here. Every position, size and line on a
 * diagram is the same in both themes, so a change of theme never moves
 * anything.
 */
export interface Theme {
    name: ThemeName;
    /** Page background, or null to leave the page transparent. */
    page: string | null;
    /** A box with nothing inside it. */
    fill: string;
    /** A box that holds other boxes, tinted apart from its contents. */
    containerFill: string;
    /** The use case view's system boundary. */
    boundaryFill: string;
    /** The two side faces of a deployment-view host. */
    node3dFill: string;
    /** Every line between boxes, and the arrowheads on them. */
    stroke: string;
    /** The outline round a box. */
    boxStroke: string;
    /** Names. */
    text: string;
    /** Stereotypes and compartment titles. */
    stereotype: string;
    /** «include», an interface on a wire: the one colour that is not grey. */
    edgeLabel: string;
    /** The source path in the frame's corner. */
    provenanceText: string;
    /** Painted behind a label so a line cannot strike through it. */
    halo: string;
    /** The process view's activation bar, on a lifeline. */
    activationFill: string;
    /** A hollow arrowhead, and a port on a box's border. */
    markerFill: string;
    /** The actor's head, filled so the figure reads at a distance. */
    actorFill: string;
    /** Rounded corners, in px; 0 for square. */
    boxRadius: number;
    /** A highlight across the top of a box, which is what reads as glass. */
    gloss: boolean;
    /** A soft shadow under a box, lifting it off the page. */
    shadow: boolean;
    /** The diagram frame, its heading pentagon and the provenance line. */
    frame: boolean;
    /** Gradients and filters the theme's fills refer to. */
    defs: string;
}

export type ThemeName = 'light' | 'dark-glass';

/** Paper: what ortho has always drawn, and what the examples commit. */
export const lightTheme: Theme = {
    name: 'light',
    page: 'white',
    fill: '#fdfdf6',
    containerFill: '#f6f6ec',
    boundaryFill: '#fbfbf1',
    node3dFill: '#f0f0e6',
    stroke: '#2b2b2b',
    boxStroke: '#2b2b2b',
    text: '#1a1a1a',
    stereotype: '#555555',
    edgeLabel: '#555555',
    provenanceText: '#8a8a86',
    halo: 'white',
    activationFill: '#f1f1e4',
    markerFill: 'white',
    actorFill: '#fdfdf6',
    boxRadius: 0,
    gloss: false,
    shadow: false,
    frame: true,
    defs: ''
};

/**
 * Slides: no page, glass boxes lifted off whatever is behind them, and one
 * accent so a diagram is not wholly grey. The frame goes with the page — a
 * slide has its own title, and the pentagon would only repeat it.
 */
export const darkGlassTheme: Theme = {
    name: 'dark-glass',
    page: null,
    fill: 'url(#glass)',
    containerFill: 'url(#glassDeep)',
    boundaryFill: 'url(#glassDeep)',
    node3dFill: 'url(#glassDeep)',
    stroke: '#A9C2D1',
    boxStroke: '#C3D5E0',
    text: '#E8EEF2',
    stereotype: '#9FB3BF',
    edgeLabel: '#6FC7D6',
    provenanceText: '#6E828E',
    halo: '#10161C',
    activationFill: 'url(#glass)',
    markerFill: '#1B242C',
    actorFill: '#C3D5E0',
    boxRadius: 5,
    gloss: true,
    shadow: true,
    frame: false,
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

/** `rx`/`ry` for a box, empty when the theme draws square corners. */
export function radiusAttrs(t: Theme, radius = t.boxRadius): string {
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
export function glossOver(t: Theme, x: number, y: number, width: number, height: number): string {
    if (!t.gloss) {
        return '';
    }
    const inset = 0.75;
    const top = Math.min(height * 0.55, 26);
    const radius = Math.max(t.boxRadius - 1, 0);
    return `
      <rect x="${x + inset}" y="${y + inset}" width="${width - inset * 2}" height="${top}"${radiusAttrs(t, radius)}
          fill="url(#gloss)" />`;
}
