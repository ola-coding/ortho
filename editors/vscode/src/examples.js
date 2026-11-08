// @ts-check
// The examples the Open example command offers, each a model the extension carries in
// examples/, copied there from ortho's own by `cargo xtask vscode-grammar`. Nothing here
// needs VS Code, so the tests run it with plain Node.
'use strict';

/** @typedef {{ name: string, label: string, detail: string }} Example */

/** @type {Example[]} */
const EXAMPLES = [
  {
    name: 'coffee-machine',
    label: 'Coffee machine',
    detail: 'All six views, from what users ask for to the board that runs each program.',
  },
  {
    name: 'survey-drone',
    label: 'Survey drone',
    detail: 'A drone and its ground tablet: a larger model, its software deployed over three computers.',
  },
];

/** The file opened first: the use case view, where a model starts. */
const FIRST_FILE = 'use-case.sysml';

/**
 * A folder name for the copy that is not taken yet: `coffee-machine`, else
 * `coffee-machine-2`, `coffee-machine-3` and so on.
 * @param {string} name
 * @param {(candidate: string) => boolean} taken
 */
function freeName(name, taken) {
  if (!taken(name)) return name;
  for (let number = 2; ; number++) {
    const candidate = `${name}-${number}`;
    if (!taken(candidate)) return candidate;
  }
}

/**
 * The file to open in a copied example: the use case view, else the first .sysml file.
 * @param {string[]} files
 */
function firstFile(files) {
  const models = files.filter((file) => file.endsWith('.sysml')).sort();
  return models.includes(FIRST_FILE) ? FIRST_FILE : models[0];
}

module.exports = { EXAMPLES, freeName, firstFile };
