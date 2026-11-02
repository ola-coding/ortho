//! How a diagram is painted. The light theme is for documents: flat, one tint on
//! white. `dark-glass` is for slides: translucent boxes on a transparent background,
//! so whatever dark colour the slide uses shows through.
//!
//! Only colours and a few flags live here. Every position, size and line on a diagram
//! is the same in both themes, so a change of theme never moves anything.
use super::number::Js;

pub struct Theme {
    /// Page background, or none to leave the page transparent.
    pub page: Option<&'static str>,
    /// A box's name compartment, which is the whole box when it holds nothing else;
    /// none to tint it no differently from the rest.
    pub name_fill: Option<&'static str>,
    /// The rest of a box.
    pub fill: &'static str,
    /// The rest of a box that holds other boxes.
    pub container_fill: &'static str,
    /// Every line between boxes, and the arrowheads on them.
    pub stroke: &'static str,
    /// The outline round a box.
    pub box_stroke: &'static str,
    /// Names.
    pub text: &'static str,
    /// Keywords and compartment titles.
    pub keyword: &'static str,
    /// «include», an interface on a wire.
    pub edge_label: &'static str,
    /// The source path in the frame's corner.
    pub provenance_text: &'static str,
    /// Painted behind a label so a line cannot strike through it, and inside a
    /// message's hollow arrowhead.
    pub halo: &'static str,
    /// A port on a box's border.
    pub port_fill: &'static str,
    /// The actor's head, filled so the figure reads at a distance.
    pub actor_fill: &'static str,
    /// A highlight across the top of a box, which is what reads as glass.
    pub gloss: bool,
    /// A soft shadow under a box, lifting it off the page.
    pub shadow: bool,
    /// What the diagram frame holds the drawing on, or none for no frame. Its outline,
    /// the tab naming the view and the corner naming the source file are painted as a
    /// box's outline and name are.
    pub frame_fill: Option<&'static str>,
    /// Gradients and filters the theme's fills refer to.
    pub defs: &'static str,
}

/// Documents, and what the examples commit: a flat tint on white, inside a pale frame.
pub const LIGHT: Theme = Theme {
    page: Some("white"),
    name_fill: Some("#ccdce5"),
    fill: "white",
    container_fill: "white",
    stroke: "#1a1a1a",
    box_stroke: "#40657e",
    text: "#1a1a1a",
    keyword: "#40657e",
    edge_label: "#40657e",
    provenance_text: "#1a1a1a",
    halo: "#f9fbfc",
    port_fill: "white",
    actor_fill: "#ccdce5",
    gloss: false,
    shadow: false,
    frame_fill: Some("#f9fbfc"),
    defs: "",
};

/// Slides: no page, glass boxes lifted off whatever is behind them, and one accent so
/// a diagram is not wholly grey. The frame goes with the page: a slide has its own
/// title, and the frame's tab would only repeat it.
pub const DARK_GLASS: Theme = Theme {
    page: None,
    name_fill: None,
    fill: "url(#glass)",
    container_fill: "url(#glassDeep)",
    stroke: "#A9C2D1",
    box_stroke: "#C3D5E0",
    text: "#E8EEF2",
    keyword: "#9FB3BF",
    edge_label: "#6FC7D6",
    provenance_text: "#6E828E",
    halo: "#10161C",
    port_fill: "#1B242C",
    actor_fill: "#C3D5E0",
    gloss: true,
    shadow: true,
    frame_fill: None,
    defs: r##"
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
    </filter>"##,
};

impl Theme {
    pub fn named(name: &str) -> Option<&'static Theme> {
        match name {
            "light" => Some(&LIGHT),
            "dark-glass" => Some(&DARK_GLASS),
            _ => None,
        }
    }

    /// The shadow attribute for a box, empty when the theme has none.
    pub(super) fn lift(&self) -> &'static str {
        if self.shadow { r#" filter="url(#lift)""# } else { "" }
    }

    /// The gloss over the top of a box: a short gradient inset from its border, so the
    /// light stops short of the outline rather than washing it out.
    pub(super) fn gloss_over(&self, x: f64, y: f64, width: f64, height: f64, radius: f64) -> String {
        if !self.gloss {
            return String::new();
        }
        let inset = 0.75;
        let top = (height * 0.55).min(26.0);
        format!(
            r#"
      <rect x="{x}" y="{y}" width="{width}" height="{top}"{radius}
          fill="url(#gloss)" />"#,
            x = Js(x + inset),
            y = Js(y + inset),
            width = Js(width - inset * 2.0),
            top = Js(top),
            radius = radius_attrs((radius - 1.0).max(0.0)),
        )
    }
}

/// `rx`/`ry` for a box, empty for square corners.
pub(super) fn radius_attrs(radius: f64) -> String {
    if radius > 0.0 { format!(r#" rx="{r}" ry="{r}""#, r = Js(radius)) } else { String::new() }
}
