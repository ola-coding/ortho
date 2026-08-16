// Public programmatic API: parse .sysml files and render 4+1 view diagrams.
export { createSysmlServices } from './parser/sysml-module.js';
export type { SysmlServices } from './parser/sysml-module.js';
export { parseSysmlFiles } from './parser/parse.js';
export type { ParsedModel } from './parser/parse.js';
export { diagramTypes, getDiagramType } from './pipeline.js';
export type { DiagramType, DiagramTypeName, DiagramRenderer, RenderResult } from './pipeline.js';
export type { DiagramTitle } from './render/svg-renderer.js';
export type { Model } from './generated/ast.js';
