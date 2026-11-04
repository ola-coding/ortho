//! Whether a diagram stays readable on landscape A4.
use ortho::pipeline::render_views;
use ortho::render::page::{PAGE_HEIGHT, PAGE_WIDTH, page_warning, scale_to_page, svg_size};
use ortho::render::theme::LIGHT;

mod common;
use common::parse_ok;

#[test]
fn a_diagram_that_fits_the_page_prints_at_its_own_size() {
    assert_eq!(scale_to_page(600.0, 400.0), 1.0);
    assert_eq!(page_warning("Small", 600.0, 400.0), None);
}

#[test]
fn a_diagram_nearly_half_again_the_page_still_reads() {
    // Names print at 9 pt unscaled; shrunk to fit a page half as big again, they
    // print at 6 pt, the least that reads.
    assert_eq!(page_warning("Wide", PAGE_WIDTH * 1.45, 300.0), None);
    assert_eq!(page_warning("Tall", 300.0, PAGE_HEIGHT * 1.45), None);
}

#[test]
fn a_diagram_twice_the_page_is_too_wide_or_too_tall_for_it() {
    let wide = page_warning("Logical view", PAGE_WIDTH * 2.0, 300.0).unwrap();
    assert!(wide.starts_with("'Logical view' is too wide for landscape A4"), "{wide}");
    assert!(wide.contains("print at 4.5 pt"), "{wide}");
    let tall = page_warning("Implementation view", 300.0, PAGE_HEIGHT * 2.0).unwrap();
    assert!(tall.starts_with("'Implementation view' is too tall for landscape A4"), "{tall}");
}

#[test]
fn reads_the_size_an_svg_declares() {
    let svg =
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"812.5\" height=\"370\"\n     viewBox=\"0 0 812.5 370\">";
    assert_eq!(svg_size(svg), (812.5, 370.0));
}

#[test]
fn warns_of_a_view_too_big_to_read_and_only_of_that_one() {
    // Leaves are listed one under another, and a list does not wrap.
    let leaves: String = (0..60).map(|i| format!("action function{i}; ")).collect();
    let workspace = parse_ok(&format!(
        "package Long {{
            action def Everything {{ {leaves} }}
            action def Little {{ action one; action two; }}
            view 'Every function' : GeneralView {{ expose Everything; }}
            view 'A few functions' : GeneralView {{ expose Little; }}
        }}"
    ));
    let rendering = render_views(&workspace, &LIGHT).unwrap();
    let warnings: Vec<&str> = rendering.rendered.iter().filter_map(|r| r.warning.as_deref()).collect();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].starts_with("'Every function' is too tall"), "{}", warnings[0]);
}
