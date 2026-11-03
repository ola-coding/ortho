//! How the renderer paints.
use ortho::layout::{LaidOutDiagram, LaidOutEdge, LaidOutNode, Point};
use ortho::model::graph::{Compartment, EdgeKind, NodeShape};
use ortho::render::number::{Js, hypot};
use ortho::render::svg::{DiagramTitle, render_frame, render_svg};
use ortho::render::theme::{DARK_GLASS, LIGHT, Theme};

fn title() -> DiagramTitle {
    DiagramTitle {
        heading: "Use case view — DroneUseCases".to_string(),
        definition: Some("GeneralView".to_string()),
        source: "fixtures/use-case.sysml".to_string(),
    }
}

fn node(shape: NodeShape, keyword: &str, name: &str) -> LaidOutNode {
    LaidOutNode {
        id: name.to_string(),
        shape,
        keyword: keyword.to_string(),
        name: name.to_string(),
        compartments: Vec::new(),
        ports: Vec::new(),
        has_children: false,
        x: 0.0,
        y: 0.0,
        width: 200.0,
        height: 90.0,
    }
}

/// «use case» boxes, an actor and an «include» label.
fn use_cases() -> LaidOutDiagram {
    LaidOutDiagram {
        width: 400.0,
        height: 200.0,
        nodes: vec![node(NodeShape::Usage, "use case", "fly"), node(NodeShape::Actor, "", "pilot")],
        edges: vec![LaidOutEdge {
            id: "e".to_string(),
            kind: EdgeKind::Include,
            source_id: "fly".to_string(),
            target_id: "pilot".to_string(),
            label: Some("«include»".to_string()),
            points: vec![Point { x: 0.0, y: 0.0 }, Point { x: 100.0, y: 0.0 }],
            label_box: None,
        }],
    }
}

fn svg_of(n: LaidOutNode, theme: &Theme) -> String {
    render_svg(&LaidOutDiagram { width: 200.0, height: 90.0, nodes: vec![n], edges: Vec::new() }, &title(), theme)
}

fn count(text: &str, pattern: &str) -> usize {
    text.matches(pattern).count()
}

#[test]
fn paints_the_light_theme_on_paper_inside_a_frame() {
    let svg = render_svg(&use_cases(), &title(), &LIGHT);
    assert!(svg.contains(r#"fill="white""#));
    // The frame's tab names the view (§8.2.3.26); its definition is optional there, and left out.
    assert!(
        svg.contains(&format!(
            r#"&#171;view&#187;</tspan><tspan font-weight="600"> {}</tspan></text>"#,
            title().heading
        ))
    );
    assert!(!svg.contains(": GeneralView"));
    assert!(svg.contains(&title().source));
    assert!(svg.contains(&format!(r#"fill="{}""#, LIGHT.name_fill.unwrap())));
    assert!(!svg.contains("url(#glass)"));
}

#[test]
fn tints_a_box_up_to_the_end_of_its_name_compartment_and_leaves_the_rest_white() {
    let box_tinted = r##"height="90" rx="10" ry="10"
          fill="#ccdce5""##;
    let box_white = r#"height="90" rx="10" ry="10"
          fill="white""#;
    let band = r##"Z" fill="#ccdce5" />"##;

    // Nothing else inside: the whole box is its name compartment.
    let alone = svg_of(node(NodeShape::Usage, "part", "a"), &LIGHT);
    assert!(alone.contains(box_tinted));
    // The frame's tab and corner are paths too, filled on a line of their own.
    assert!(!alone.contains(band));

    // With compartments or nested boxes, the band ends where the name does.
    let mut with_lines = node(NodeShape::Usage, "part", "a");
    with_lines.compartments.push(Compartment { title: None, lines: vec!["x".to_string()] });
    let mut with_children = node(NodeShape::Usage, "part", "a");
    with_children.has_children = true;
    for n in [with_lines, with_children] {
        let svg = svg_of(n, &LIGHT);
        assert!(svg.contains(band));
        assert!(svg.contains(box_white));
    }

    // A theme without a tint of its own draws no band.
    let mut nested = node(NodeShape::Usage, "part", "a");
    nested.has_children = true;
    let dark = svg_of(nested, &DARK_GLASS);
    assert!(!dark.contains("#ccdce5"));
    let after_defs = dark.split("</defs>").nth(1).unwrap();
    assert!(!after_defs.contains("<path"));
}

#[test]
fn paints_the_frame_as_the_boxes_inside_it_are_painted() {
    let frame = render_frame(400.0, 300.0, &title(), &LIGHT);
    assert!(frame.contains(&format!(r#"fill="{}""#, LIGHT.frame_fill.unwrap())));
    // The tab and the corner take a box's tint, all three a box's outline.
    assert_eq!(count(&frame, &format!(r#"fill="{}""#, LIGHT.name_fill.unwrap())), 2);
    assert_eq!(count(&frame, &format!(r#"stroke="{}""#, LIGHT.box_stroke)), 3);
    assert_eq!(render_frame(400.0, 300.0, &title(), &DARK_GLASS), "");
}

#[test]
fn paints_dark_glass_on_nothing_at_all_for_a_slide_to_show_through() {
    let svg = render_svg(&use_cases(), &title(), &DARK_GLASS);

    // No page: the slide is the background.
    assert!(!svg.contains(r#"fill="white""#));
    // No frame and no corner text: the slide has its own title. The <title> element
    // keeps both, so the file still names its source.
    assert!(!svg.contains("&#171;view&#187;"));
    assert!(!svg.contains(&format!(">{}<", title().source)));
    assert!(svg.contains(&format!("<title>{} (generated from {})</title>", title().heading, title().source)));

    // Glass, lifted, with a gloss over the top.
    assert!(svg.contains(r#"<linearGradient id="glass""#));
    assert!(svg.contains(r#"fill="url(#glass)""#) || svg.contains(r#"fill="url(#glassDeep)""#));
    assert!(svg.contains(r#"fill="url(#gloss)""#));
    assert!(svg.contains(r#"filter="url(#lift)""#));
    // A line round every box, and one accent colour on the edge labels.
    assert!(svg.contains(&format!(r#"stroke="{}""#, DARK_GLASS.box_stroke)));
    assert!(svg.contains(&format!(r#"fill="{}""#, DARK_GLASS.edge_label)));
    // The actor keeps a filled head, so the figure reads on a slide.
    assert!(svg.contains(&format!(r#"r="8" fill="{}""#, DARK_GLASS.actor_fill)));
}

/// The value of each position attribute in an SVG, once each.
fn positions(svg: &str) -> Vec<String> {
    let mut found = Vec::new();
    for attribute in ["x", "y", "cx", "cy", "points"] {
        for part in svg.split(&format!(" {attribute}=\"")).skip(1) {
            let value = format!("{attribute}={}", part.split('"').next().unwrap());
            if !found.contains(&value) {
                found.push(value);
            }
        }
    }
    found
}

#[test]
fn moves_nothing_both_themes_lay_the_diagram_out_identically() {
    // Without frame and page, so their own coordinates are not in the way.
    let frameless = Theme { frame_fill: None, page: None, ..LIGHT };
    let light = positions(&render_svg(&use_cases(), &title(), &frameless));
    let dark = positions(&render_svg(&use_cases(), &title(), &DARK_GLASS));
    // The dark theme adds the gloss rectangles, so compare what the light theme drew.
    assert!(!light.is_empty());
    for position in light {
        assert!(dark.contains(&position), "{position}");
    }
}

#[test]
fn names_an_unnamed_view_by_its_definition() {
    let unnamed = DiagramTitle {
        heading: String::new(),
        definition: Some("GeneralView".to_string()),
        source: "inline".to_string(),
    };
    let frame = render_frame(400.0, 300.0, &unnamed, &LIGHT);
    assert!(frame.contains(r#"&#171;view&#187;</tspan><tspan font-weight="600"></tspan> : GeneralView</text>"#));
}

#[test]
fn knows_its_themes_by_name() {
    assert_eq!(Theme::named("light").unwrap().page, LIGHT.page);
    assert_eq!(Theme::named("dark-glass").unwrap().stroke, DARK_GLASS.stroke);
    assert!(Theme::named("midnight").is_none());
}

#[test]
fn writes_numbers_as_javascript_does() {
    let written = |x: f64| Js(x).to_string();
    assert_eq!(written(24.0), "24");
    assert_eq!(written(-0.0), "0");
    assert_eq!(written(1083.7526666666668), "1083.7526666666668");
    assert_eq!(written(0.1 + 0.2), "0.30000000000000004");
    assert_eq!(written(1e21), "1e+21");
    assert_eq!(written(1.5e21), "1.5e+21");
    assert_eq!(written(1e20), "100000000000000000000");
    assert_eq!(written(0.000001), "0.000001");
    assert_eq!(written(1.5e-7), "1.5e-7");
    assert_eq!(written(-2e-7), "-2e-7");
    assert_eq!(written(f64::INFINITY), "Infinity");
    assert_eq!(written(f64::NEG_INFINITY), "-Infinity");
    assert_eq!(written(f64::NAN), "NaN");
}

#[test]
fn takes_a_hypotenuse_as_v8_does() {
    // What Math.hypot gave in Node 24.
    assert_eq!(hypot(0.0, 0.0), 0.0);
    assert_eq!(hypot(3.0, 4.0), 5.0);
    assert_eq!(hypot(-3.0, 4.0), 5.0);
    assert_eq!(hypot(3.3, 7.7), 8.3773504164503);
    assert_eq!(hypot(38.6, 5.4), 38.975889983424366);
    assert_eq!(hypot(58.8, 64.2), 87.05791175993139);
    assert_eq!(hypot(0.2, 271.628), 271.62807363010177);
    assert_eq!(hypot(313.859, 270.41379), 414.28382746996544);
}
