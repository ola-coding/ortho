//! ortho renders the six architectural views as SVG from SysML v2 model files.
//! A model goes through `parser`, `model`, `diagrams`, `layout` and `render` in turn;
//! `pipeline` runs them for a view.
pub mod diagrams;
pub mod layout;
pub mod model;
pub mod parser;
pub mod pipeline;
pub mod render;
