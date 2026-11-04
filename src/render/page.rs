//! Whether a diagram stays readable on the page it is made for: landscape A4.
//!
//! An SVG scales to fit its page, so a diagram too big for the page is not cut off
//! but shrunk, and its text with it. ortho draws names at 12 px; on paper, a CSS
//! pixel is 1/96 inch, so a name prints at 9 pt times the scale.

/// The printable part of landscape A4, 297 × 210 mm less 10 mm margins, in pixels.
pub const PAGE_WIDTH: f64 = 277.0 / MM_PER_PX;
pub const PAGE_HEIGHT: f64 = 190.0 / MM_PER_PX;
const MM_PER_PX: f64 = 25.4 / 96.0;

/// A name's size on paper, unscaled: 12 px is 9 pt.
const NAME_POINTS: f64 = 9.0;
/// The smallest a name may print and still be read.
pub const MIN_NAME_POINTS: f64 = 6.0;

/// The scale a diagram of this size is printed at to fit the page; diagrams that
/// fit are printed at their own size, never enlarged.
pub fn scale_to_page(width: f64, height: f64) -> f64 {
    (PAGE_WIDTH / width).min(PAGE_HEIGHT / height).min(1.0)
}

/// Why a diagram of this size is unreadable on landscape A4, or nothing when it reads.
pub fn page_warning(heading: &str, width: f64, height: f64) -> Option<String> {
    let scale = scale_to_page(width, height);
    let points = NAME_POINTS * scale;
    if points >= MIN_NAME_POINTS {
        return None;
    }
    let too = if PAGE_WIDTH / width < PAGE_HEIGHT / height { "wide" } else { "tall" };
    Some(format!(
        "'{heading}' is too {too} for landscape A4 ({} × {} px): its names would print at {points:.1} pt. \
         Split the view into smaller ones.",
        width.round(),
        height.round()
    ))
}

/// The size an SVG of ortho's declares on its root element.
pub fn svg_size(svg: &str) -> (f64, f64) {
    let attribute = |name: &str| {
        let start = svg.find(&format!(" {name}=\"")).map(|at| at + name.len() + 3)?;
        let end = svg[start..].find('"')? + start;
        svg[start..end].parse::<f64>().ok()
    };
    (attribute("width").unwrap_or(0.0), attribute("height").unwrap_or(0.0))
}
