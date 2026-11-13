// @ts-check
// The AI instructions the Add AI instructions command writes into a workspace: a skill
// for Claude Code, instructions for GitHub Copilot, and an AGENTS.md for every other
// agent, each telling an agent how to write a model ortho draws and to check its work
// with ortho. What they say is written once, in ortho's ai/ folder and its LANGUAGE.md
// and VIEWS.md, which the extension carries in its own ai/; only how they are put
// together, and where this editor keeps ortho, is here. Nothing here needs VS Code, so
// the tests run it with plain Node.
'use strict';

const SKILL = 'sysml-v2-ortho';
const SKILL_FOLDER = `.claude/skills/${SKILL}`;

/** When the skill is used: whenever an agent works on a model, or is asked for one. */
const DESCRIPTION =
  'Write, change and review SysML v2 models, .sysml files, that ortho draws as architecture views, and check them with ortho. ' +
  'Use whenever you create, edit or review a .sysml file, or are asked to model a system in SysML v2: ' +
  'its use cases, functions, software, hardware or deployment.';

/** The reference files the skill carries beside it, by name. */
const REFERENCES = ['LANGUAGE.md', 'VIEWS.md', 'MISTAKES.md'];

/**
 * Files that, when a workspace has one, already hold its agents' instructions. Zed
 * reads the first of them it finds, so an AGENTS.md written beside a CLAUDE.md would
 * hide it there: AGENTS.md is written only when none is there.
 */
const AGENT_FILES = ['AGENTS.md', 'AGENT.md', 'CLAUDE.md', 'GEMINI.md'];

/**
 * @typedef {{ LANGUAGE: string, VIEWS: string, WORKFLOW: string, MISTAKES: string, AGENTS: string }} Sources
 * The files of the extension's ai/ folder, by name without `.md`.
 * @typedef {{ path: string, content: string, unlessAny?: string[] }} InstructionFile
 * `unlessAny`: write it only if none of these is in the workspace, and never replace it.
 */

/** Where ortho is from an agent in VS Code, and what the extension offers it. */
const EDITOR = `
## Where ortho is

Run \`ortho\`. In a VS Code terminal, the Ortho extension puts the ortho it runs first on the PATH. If \`ortho\` is not found, look in the \`bin\` folder of the Ortho extension, under \`~/.vscode/extensions/\` (\`~/.cursor/extensions/\` in Cursor), then try \`~/.cargo/bin/ortho\` or the path in the VS Code setting \`ortho.path\`. To install it: \`cargo install ortho\`, or download it from https://github.com/ola-coding/ortho/releases.

The extension's *Ortho: Open Example* command copies either example model into the workspace.
`;

/** A copied file without the comment that says where it was copied from. */
function body(text) {
  return text.replace(/^<!--.*?-->\n\n/, '');
}

/**
 * The language reference, a level deeper, to follow the instructions in one file:
 * its title becomes a section. A heading inside a code block is SysML, and stays.
 * @param {string} text
 */
function nested(text) {
  let fenced = false;
  return text
    .split('\n')
    .map((line) => {
      if (line.startsWith('```')) fenced = !fenced;
      return !fenced && /^#{1,5} /.test(line) ? `#${line}` : line;
    })
    .join('\n');
}

/**
 * Text written outside the skill's folder: a link to one of the files beside the
 * skill points into that folder, from `.github/instructions/`.
 * @param {string} text
 */
function fromInstructions(text) {
  const names = ['WORKFLOW.md', ...REFERENCES].join('|').replace(/\./g, '\\.');
  return text.replace(new RegExp(`\\]\\((${names})([#)])`, 'g'), `](../../${SKILL_FOLDER}/$1$2`);
}

/**
 * The files to write, by path from the workspace folder.
 * @param {Sources} ai
 * @returns {InstructionFile[]}
 */
function instructionFiles(ai) {
  const workflow = body(ai.WORKFLOW);
  const skill = `---
name: ${SKILL}
description: ${DESCRIPTION}
---

# SysML v2 models that ortho draws

${workflow}${EDITOR}`;
  const copilot = `---
applyTo: "**/*.sysml"
---

# SysML v2 models that ortho draws

${fromInstructions(workflow)}${EDITOR}
${nested(fromInstructions(body(ai.LANGUAGE)))}
${nested(fromInstructions(body(ai.MISTAKES)))}`;
  return [
    { path: `${SKILL_FOLDER}/SKILL.md`, content: skill },
    ...REFERENCES.map((name) => ({
      path: `${SKILL_FOLDER}/${name}`,
      content: ai[/** @type {keyof Sources} */ (name.replace('.md', ''))],
    })),
    { path: '.github/instructions/sysml-v2.instructions.md', content: copilot },
    { path: 'AGENTS.md', content: body(ai.AGENTS), unlessAny: AGENT_FILES },
  ];
}

module.exports = { instructionFiles, DESCRIPTION, SKILL };
