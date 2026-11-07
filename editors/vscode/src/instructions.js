// @ts-check
// The AI instructions the Add AI instructions command writes into a workspace: a skill
// for Claude Code and instructions for GitHub Copilot, each telling an agent how to
// write a model ortho draws and to check its work with ortho. Nothing here needs VS
// Code, so the tests run it with plain Node.
'use strict';

const SKILL = 'sysml-v2-ortho';

/** @typedef {{ path: string, content: string }} InstructionFile */

/**
 * What an agent does with a model, in either tool. `language` says where the
 * language reference is, as the sentence's object: `LANGUAGE.md, beside this file`.
 * @param {string} language
 */
function workflow(language) {
  return `The \`.sysml\` files in a folder are one SysML v2 model. ortho reads them together and draws, as SVG, every view the model declares. ${language} says what ortho reads and what each view draws from it. Read it before you change a model, and write only what it describes.

## Check your work with ortho

After every change, render the model's folder and read everything ortho prints:

\`\`\`sh
ortho render <model folder> -o <a temporary folder>
\`\`\`

Write the diagrams into the model's own folder only when asked to update them.

- **Errors**, under \`Failed to parse:\`, are what SysML v2 forbids, or a name that resolves to nothing. ortho draws nothing until they are fixed.
- **Warnings**, each starting \`Warning:\`, are views of the model that disagree: a function no part or module performs, a use case no function tree refines, software that no deployment allocates to a host. The model draws, but it is not finished.

Each problem names its file and line. Fix every error and every warning before you finish. If you leave a warning, say which one and why.

## Fix the cause, not the line

Never delete or comment out the line a problem points at to make it go away. A \`perform\`, a \`#refinement dependency\` or an \`allocate\` that no longer resolves means a name changed: find the new name, which the error suggests in a chain such as \`beverage.prepareCoffee.storeBeans\`, and update the reference. A deleted line leaves a function with nothing to realize it, or a use case nothing details.

When you rename an element, update every reference to it in every file of the folder, then render again.

## Where ortho is

Run \`ortho\`. If it is not found, try \`~/.cargo/bin/ortho\`, or the path in the VS Code setting \`ortho.path\`. To install it: \`cargo install ortho\`, or download it from https://github.com/ola-coding/ortho/releases.
`;
}

/**
 * The language reference, a level deeper, to follow the instructions in one file:
 * its title becomes a section. A heading inside a code block is SysML, and stays.
 * @param {string} language
 */
function nested(language) {
  let fenced = false;
  return language
    .split('\n')
    .map((line) => {
      if (line.startsWith('```')) fenced = !fenced;
      return !fenced && /^#{1,5} /.test(line) ? `#${line}` : line;
    })
    .join('\n');
}

/**
 * The files to write, by path from the workspace folder, from ortho's LANGUAGE.md.
 * @param {string} language
 * @returns {InstructionFile[]}
 */
function instructionFiles(language) {
  const skill = `---
name: ${SKILL}
description: Write and change SysML v2 models, .sysml files, that ortho draws as architecture views, and check them with ortho. Use whenever you create or edit a .sysml file.
---

# SysML v2 models that ortho draws

${workflow('[LANGUAGE.md](LANGUAGE.md), beside this file,')}`;
  const copilot = `---
applyTo: "**/*.sysml"
---

# SysML v2 models that ortho draws

${workflow('The language reference below')}
${nested(language)}`;
  return [
    { path: `.claude/skills/${SKILL}/SKILL.md`, content: skill },
    { path: `.claude/skills/${SKILL}/LANGUAGE.md`, content: language },
    { path: '.github/instructions/sysml-v2.instructions.md', content: copilot },
  ];
}

module.exports = { instructionFiles };
