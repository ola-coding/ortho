// @ts-check
// Reads the Markdown ortho writes for a table of requirements: headings, paragraphs,
// tables, links, line breaks, a <sub> line and character references, each written the
// way ortho writes it. It is not a general Markdown reader. The preview page builds its
// tab from what this returns, and the tests read the examples' tables with it.
(function (root, factory) {
  if (typeof module === 'object' && module.exports) module.exports = factory();
  // @ts-ignore the preview page has no modules
  else root.orthoMarkdown = factory();
})(this, function () {
  'use strict';

  /**
   * @typedef {{ type: 'text', text: string } | { type: 'br' } | { type: 'link', href: string, children: Inline[] } | { type: 'sub', children: Inline[] }} Inline
   * @typedef {{ type: 'heading', level: number, id: string, children: Inline[] } | { type: 'paragraph', children: Inline[] } | { type: 'table', head: Inline[][], rows: Inline[][][] }} Block
   */

  /** @type {Record<string, string>} */
  const NAMED = { emsp: ' ', nbsp: ' ', amp: '&', lt: '<', gt: '>', quot: '"' };

  /**
   * A line of Markdown as text, links, line breaks and <sub>: `\x` is `x` as written.
   * @param {string} source
   * @returns {Inline[]}
   */
  function inline(source) {
    /** @type {Inline[]} */
    const top = [];
    /** @type {Inline[][]} */
    const open = [top];
    let text = '';
    const here = () => /** @type {Inline[]} */ (open[open.length - 1]);
    const flush = () => {
      if (text) here().push({ type: 'text', text });
      text = '';
    };
    let i = 0;
    while (i < source.length) {
      const rest = source.slice(i);
      if (rest[0] === '\\' && rest.length > 1) {
        text += rest[1];
        i += 2;
        continue;
      }
      if (rest[0] === '[') {
        const link = linkAt(rest);
        if (link) {
          flush();
          here().push({ type: 'link', href: link.href, children: inline(link.label) });
          i += link.length;
          continue;
        }
      }
      const tag = /^<(\/?)(sub|br)\s*\/?>/i.exec(rest);
      if (tag) {
        flush();
        if (tag[2].toLowerCase() === 'br') here().push({ type: 'br' });
        else if (!tag[1]) {
          /** @type {Inline} */
          const sub = { type: 'sub', children: [] };
          here().push(sub);
          open.push(sub.children);
        } else if (open.length > 1) open.pop();
        i += tag[0].length;
        continue;
      }
      const reference = /^&(?:#(\d+)|#x([0-9a-f]+)|([a-z]+));/i.exec(rest);
      if (reference) {
        const code = reference[1] ? Number(reference[1]) : reference[2] ? parseInt(reference[2], 16) : undefined;
        const character = code !== undefined ? String.fromCodePoint(code) : NAMED[reference[3].toLowerCase()];
        if (character !== undefined) {
          text += character;
          i += reference[0].length;
          continue;
        }
      }
      text += rest[0];
      i += 1;
    }
    flush();
    return top;
  }

  /**
   * The link `[label](href)` at the start of `source`, if one is there.
   * @param {string} source
   */
  function linkAt(source) {
    let depth = 0;
    for (let i = 0; i < source.length; i++) {
      const c = source[i];
      if (c === '\\') {
        i++;
      } else if (c === '[') {
        depth++;
      } else if (c === ']') {
        depth--;
        if (depth === 0) {
          const target = /^\(([^)\s]*)\)/.exec(source.slice(i + 1));
          if (!target) return undefined;
          return { label: source.slice(1, i), href: target[1], length: i + 1 + target[0].length };
        }
      }
    }
    return undefined;
  }

  /**
   * The cells of a table row, split at each `|` not escaped.
   * @param {string} line
   */
  function cells(line) {
    const inner = line.trim().replace(/^\|/, '').replace(/(?<!\\)\|$/, '');
    /** @type {string[]} */
    const found = [];
    let cell = '';
    for (let i = 0; i < inner.length; i++) {
      if (inner[i] === '\\' && i + 1 < inner.length) {
        cell += inner[i] + inner[i + 1];
        i++;
      } else if (inner[i] === '|') {
        found.push(cell.trim());
        cell = '';
      } else {
        cell += inner[i];
      }
    }
    found.push(cell.trim());
    return found;
  }

  /**
   * The text an inline shows, as a heading's anchor is made from it.
   * @param {Inline[]} children
   * @returns {string}
   */
  function plainText(children) {
    return children
      .map((child) => (child.type === 'text' ? child.text : child.type === 'br' ? ' ' : plainText(child.children)))
      .join('');
  }

  /**
   * A heading's anchor, as GitHub makes it and ortho links to it: lower case, spaces
   * as hyphens, only letters, digits, hyphens and underscores kept, and a number after
   * one already taken.
   */
  function anchors() {
    /** @type {Map<string, number>} */
    const taken = new Map();
    return (/** @type {string} */ heading) => {
      const slug = [...heading.toLowerCase()]
        .map((c) => (c === ' ' ? '-' : c === '-' || c === '_' || /[\p{Alphabetic}\p{N}]/u.test(c) ? c : ''))
        .join('');
      const count = taken.get(slug) ?? 0;
      taken.set(slug, count + 1);
      return count === 0 ? slug : `${slug}-${count}`;
    };
  }

  /**
   * The blocks of a table of requirements, in order.
   * @param {string} markdown
   * @returns {Block[]}
   */
  function parse(markdown) {
    const lines = markdown.replace(/\r\n?/g, '\n').split('\n');
    const anchor = anchors();
    /** @type {Block[]} */
    const blocks = [];
    let i = 0;
    while (i < lines.length) {
      const line = lines[i];
      if (!line.trim()) {
        i++;
        continue;
      }
      const heading = /^(#{1,6})\s+(.*)$/.exec(line);
      if (heading) {
        const children = inline(heading[2].trim());
        blocks.push({ type: 'heading', level: heading[1].length, id: anchor(plainText(children)), children });
        i++;
        continue;
      }
      if (line.trimStart().startsWith('|')) {
        /** @type {string[]} */
        const rows = [];
        while (i < lines.length && lines[i].trimStart().startsWith('|')) rows.push(lines[i++]);
        const [head = '', , ...body] = rows;
        blocks.push({
          type: 'table',
          head: cells(head).map(inline),
          rows: body.map((row) => cells(row).map(inline)),
        });
        continue;
      }
      /** @type {string[]} */
      const paragraph = [];
      while (i < lines.length && lines[i].trim() && !/^#{1,6}\s/.test(lines[i]) && !lines[i].trimStart().startsWith('|')) {
        paragraph.push(lines[i++].trim());
      }
      blocks.push({ type: 'paragraph', children: inline(paragraph.join(' ')) });
    }
    return blocks;
  }

  return { parse, inline, plainText };
});
