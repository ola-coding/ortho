//! Deterministic text measurement without a browser or font files: advance widths
//! (1/1000 em) for ASCII 32-126 of the two fonts the SVG's font list resolves to on
//! Windows, Linux and Typst. Windows (and macOS) draw Arial; Linux draws Liberation
//! Sans, which has Arial's widths, or DejaVu Sans; Typst matches the same names, and
//! without them uses the document's font, which for its defaults is no wider than
//! Arial. Each character is measured in the wider of Arial and DejaVu Sans, so text
//! fits whichever one is drawn.

#[rustfmt::skip]
const ARIAL_REGULAR: [u32; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278,
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556,
    1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778,
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556,
    333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556,
    556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

#[rustfmt::skip]
const ARIAL_BOLD: [u32; 95] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278,
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611,
    975, 722, 722, 722, 722, 667, 611, 778, 722, 278, 556, 722, 611, 833, 722, 778,
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 333, 278, 333, 584, 556,
    333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556, 278, 889, 611, 611,
    611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584,
];

#[rustfmt::skip]
const DEJAVU_REGULAR: [u32; 95] = [
    318, 401, 460, 838, 636, 950, 780, 275, 390, 390, 500, 838, 318, 361, 318, 337,
    636, 636, 636, 636, 636, 636, 636, 636, 636, 636, 337, 337, 838, 838, 838, 531,
    1000, 684, 686, 698, 770, 632, 575, 775, 752, 295, 295, 656, 557, 863, 748, 787,
    603, 787, 695, 635, 611, 732, 684, 989, 685, 611, 685, 390, 337, 390, 838, 500,
    500, 613, 635, 550, 635, 615, 352, 635, 634, 278, 278, 579, 278, 974, 634, 612,
    635, 635, 411, 521, 392, 634, 592, 818, 592, 592, 525, 636, 337, 636, 838,
];

#[rustfmt::skip]
const DEJAVU_BOLD: [u32; 95] = [
    348, 456, 521, 838, 696, 1002, 872, 306, 457, 457, 523, 838, 380, 415, 380, 365,
    696, 696, 696, 696, 696, 696, 696, 696, 696, 696, 400, 400, 838, 838, 838, 580,
    1000, 774, 762, 734, 830, 683, 683, 821, 837, 372, 372, 775, 637, 995, 837, 850,
    733, 850, 770, 720, 682, 812, 774, 1103, 771, 724, 725, 457, 365, 457, 838, 500,
    500, 675, 716, 593, 716, 678, 435, 716, 712, 343, 343, 665, 343, 1042, 712, 687,
    716, 716, 493, 595, 478, 712, 652, 924, 645, 652, 582, 712, 365, 712, 838,
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    Regular,
    Bold,
}

pub fn measure_text(text: &str, font_size: f64, weight: Weight) -> f64 {
    let (arial, dejavu) = match weight {
        Weight::Regular => (&ARIAL_REGULAR, &DEJAVU_REGULAR),
        Weight::Bold => (&ARIAL_BOLD, &DEJAVU_BOLD),
    };
    let mut units = 0;
    for c in text.chars() {
        units += match (c as u32, weight) {
            (code @ 32..=126, _) => arial[code as usize - 32].max(dejavu[code as usize - 32]),
            // « », DejaVu Sans
            (0xab | 0xbb, Weight::Bold) => 646,
            (0xab | 0xbb, Weight::Regular) => 612,
            // unknown: a generous average
            (_, Weight::Bold) => 760,
            (_, Weight::Regular) => 700,
        };
    }
    f64::from(units) * font_size / 1000.0
}
