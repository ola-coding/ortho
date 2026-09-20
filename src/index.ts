// Public programmatic API: parse .sysml files and render the six architectural views.
export { createSysmlServices } from './parser/sysml-module.js';
export type { SysmlServices } from './parser/sysml-module.js';
export { parseSysmlFiles } from './parser/parse.js';
export type { ParsedModel } from './parser/parse.js';
export { diagramTypes, getDiagramType } from './pipeline.js';
export type { DiagramType, DiagramTypeName, DiagramRenderer, RenderResult } from './pipeline.js';
export type { DiagramTitle } from './render/svg-renderer.js';
export { darkGlassTheme, getTheme, lightTheme, themes } from './render/theme.js';
export type { Theme, ThemeName } from './render/theme.js';
export type { Model } from './generated/ast.js';
