// @ts-check
// The preview panel's page: a tab for each view ortho drew, the chosen view's diagram or
// table of requirements, and what ortho reported. The extension sends each render as a
// message.
(function () {
  // @ts-ignore acquireVsCodeApi is given to webviews by VS Code
  const vscode = acquireVsCodeApi();
  const tabs = /** @type {HTMLElement} */ (document.getElementById('tabs'));
  const status = /** @type {HTMLElement} */ (document.getElementById('status'));
  const messages = /** @type {HTMLElement} */ (document.getElementById('messages'));
  const diagram = /** @type {HTMLElement} */ (document.getElementById('diagram'));
  const actions = /** @type {HTMLElement} */ (document.getElementById('actions'));
  const saveOne = /** @type {HTMLButtonElement} */ (document.getElementById('save-one'));
  const saveAll = /** @type {HTMLButtonElement} */ (document.getElementById('save-all'));
  saveOne.addEventListener('click', () => vscode.postMessage({ type: 'save' }));
  saveAll.addEventListener('click', () => vscode.postMessage({ type: 'saveAll' }));

  /** @type {'none' | 'current' | 'stale'} */
  let shown = 'none';

  /**
   * The save buttons: hidden with no views, and disabled while the model does not
   * load, since the views shown then no longer match it. The first saves the view
   * shown: a diagram as SVG, or the table of requirements as Markdown.
   * @param {'none' | 'current' | 'stale'} [state]
   */
  function showActions(state = shown) {
    shown = state;
    actions.hidden = state === 'none';
    const why = state === 'stale' ? 'The model does not load now: fix its problems to save its views' : '';
    const markdown = Boolean(selectedView()?.markdown);
    saveOne.textContent = markdown ? 'Save Markdown…' : 'Save SVG…';
    saveOne.disabled = saveAll.disabled = state === 'stale';
    saveOne.title = why || (markdown ? 'Save the table shown as a Markdown file' : 'Save the view shown as an SVG file');
    saveAll.title = why || 'Save every view into a folder';
  }

  /** @type {{ folder?: string, selected?: string, actualSize?: boolean }} */
  const state = vscode.getState() || {};
  /** @type {{ file: string, title: string, heading: string, summary: string, svg: string, markdown: string }[]} */
  let views = [];
  /**
   * Where each requirement is declared, by its ID, or by its name for one without.
   * @type {{ byId: Record<string, { file: string, line: number }>, byName: Record<string, { file: string, line: number }> }}
   */
  let requirements = { byId: {}, byName: {} };

  function selectedView() {
    return views.find((v) => v.title === state.selected) || views[0];
  }

  function save() {
    vscode.setState(state);
  }

  /** @param {string} tag @param {Record<string, string>} [attributes] @param {string} [text] */
  function element(tag, attributes = {}, text = '') {
    const node = document.createElement(tag);
    for (const [name, value] of Object.entries(attributes)) node.setAttribute(name, value);
    node.textContent = text;
    return node;
  }

  /** @param {string} text @param {string} kind */
  function message(text, kind) {
    const node = element('p', { class: `message ${kind}` }, text);
    messages.append(node);
    return node;
  }

  function showSelected() {
    const view = selectedView();
    for (const button of tabs.querySelectorAll('button')) {
      button.setAttribute('aria-selected', String(button.textContent === view?.title));
    }
    diagram.replaceChildren();
    diagram.classList.toggle('actual-size', Boolean(state.actualSize) && !view?.markdown);
    showActions();
    if (!view) return;
    // The extension saves the view on screen.
    vscode.postMessage({ type: 'showing', file: view.file });
    if (view.markdown) {
      diagram.append(tableOfRequirements(view));
      return;
    }
    const parsed = new DOMParser().parseFromString(view.svg, 'image/svg+xml').documentElement;
    if (parsed.nodeName !== 'svg') {
      diagram.append(element('p', { class: 'empty' }, `${view.heading} could not be read.`));
      return;
    }
    const svg = /** @type {SVGSVGElement} */ (document.importNode(parsed, true));
    svg.setAttribute('role', 'img');
    svg.setAttribute('aria-label', view.heading);
    for (const box of svg.querySelectorAll('[data-line]')) {
      const file = box.getAttribute('data-file') || '';
      const tip = document.createElementNS('http://www.w3.org/2000/svg', 'title');
      tip.textContent = `Open ${file.split('/').pop()} line ${box.getAttribute('data-line')}`;
      box.prepend(tip);
    }
    svg.addEventListener('click', (event) => {
      // A box opens the line that declares it; anywhere else switches between
      // fitting the panel and actual size.
      const box = /** @type {Element} */ (event.target).closest('[data-line]');
      if (box) {
        vscode.postMessage({ type: 'open', file: box.getAttribute('data-file'), line: box.getAttribute('data-line') });
        return;
      }
      state.actualSize = !state.actualSize;
      save();
      diagram.classList.toggle('actual-size', state.actualSize);
    });
    diagram.append(svg, element('p', { class: 'summary' }, view.summary));
  }

  /**
   * A table of requirements as GitHub shows its Markdown: a box under each
   * requirement's heading, which opens the line that declares it, and links that open
   * the model's lines or move to a requirement in the table.
   * @param {{ heading: string, markdown: string }} view
   */
  function tableOfRequirements(view) {
    // @ts-ignore markdown.js, loaded before this script, sets it
    const reader = window.orthoMarkdown;
    const blocks = reader.parse(view.markdown);
    const page = element('article', { class: 'doc', 'aria-label': view.heading });
    for (let i = 0; i < blocks.length; i++) {
      const block = blocks[i];
      const next = blocks[i + 1];
      if (block.type === 'heading' && next?.type === 'table' && reader.plainText(next.head[1] ?? []) === 'Statement') {
        page.append(requirementBox(reader, block, next));
        i++;
      } else {
        page.append(blockNode(block));
      }
    }
    return page;
  }

  /**
   * A requirement's heading and box, which open the line that declares it, found by
   * its ID or, without one, by its name.
   * @param {any} reader
   * @param {any} heading
   * @param {any} table
   */
  function requirementBox(reader, heading, table) {
    const box = element('section', { class: 'requirement' });
    box.append(blockNode(heading), blockNode(table));
    const id = reader.plainText(table.rows[0]?.[0] ?? []);
    const at = requirements.byId[id] ?? requirements.byName[reader.plainText(heading.children)];
    if (!at) return box;
    box.classList.add('opens');
    box.title = `Open ${at.file.split('/').pop()} line ${at.line}`;
    box.addEventListener('click', (event) => {
      // Text selected to copy it, or a link, is not a click on the box.
      if (String(window.getSelection() ?? '') || /** @type {Element} */ (event.target).closest('a')) return;
      vscode.postMessage({ type: 'open', file: at.file, line: String(at.line) });
    });
    return box;
  }

  /** @param {any} block */
  function blockNode(block) {
    if (block.type === 'heading') {
      const node = element(`h${block.level}`, { 'data-anchor': block.id });
      node.append(...inlineNodes(block.children));
      return node;
    }
    if (block.type === 'paragraph') {
      const node = element('p');
      node.append(...inlineNodes(block.children));
      return node;
    }
    const table = element('table');
    const head = element('tr');
    for (const cell of block.head) {
      const th = element('th');
      th.append(...inlineNodes(cell));
      head.append(th);
    }
    const body = element('tbody');
    for (const row of block.rows) {
      const tr = element('tr');
      for (const cell of row) {
        const td = element('td');
        td.append(...inlineNodes(cell));
        tr.append(td);
      }
      body.append(tr);
    }
    const thead = element('thead');
    thead.append(head);
    table.append(thead, body);
    return table;
  }

  /**
   * @param {any[]} children
   * @returns {Node[]}
   */
  function inlineNodes(children) {
    return children.map((child) => {
      if (child.type === 'text') return document.createTextNode(child.text);
      if (child.type === 'br') return element('br');
      if (child.type === 'sub') {
        const sub = element('sub');
        sub.append(...inlineNodes(child.children));
        return sub;
      }
      const link = element('a', { href: '#' });
      link.append(...inlineNodes(child.children));
      const href = String(child.href);
      if (href.startsWith('#')) {
        link.title = `Go to ${link.textContent}`;
        link.addEventListener('click', (event) => {
          event.preventDefault();
          const target = diagram.querySelector(`[data-anchor="${CSS.escape(href.slice(1))}"]`);
          if (!target) return;
          target.scrollIntoView({ behavior: 'smooth', block: 'start' });
          const box = target.closest('.requirement') ?? target;
          box.animate([{ background: 'var(--vscode-editor-findMatchHighlightBackground)' }, { background: 'transparent' }], 1200);
        });
      } else {
        const place = /^([^#]*)(?:#L(\d+))?$/.exec(href);
        const file = decodeURIComponent(place?.[1] ?? href).split('/').pop();
        link.title = place?.[2] ? `Open ${file} line ${place[2]}` : `Open ${file}`;
        link.addEventListener('click', (event) => {
          event.preventDefault();
          vscode.postMessage({ type: 'openLink', href });
        });
      }
      return link;
    });
  }

  function showTabs() {
    tabs.replaceChildren(
      ...views.map((view) => {
        const button = element('button', { role: 'tab', type: 'button', title: view.heading }, view.title);
        button.addEventListener('click', () => {
          state.selected = view.title;
          save();
          showSelected();
        });
        return button;
      }),
    );
  }

  window.addEventListener('message', (event) => {
    const data = event.data;
    if (data.type === 'busy') {
      status.textContent = 'Rendering…';
    } else if (data.type === 'missing') {
      status.textContent = '';
      showActions('none');
      messages.replaceChildren();
      const node = message(
        `ortho was not found ("${data.ortho}"). Install it, or set ortho.path to where it is.`,
        'error',
      );
      const install = element('button', { type: 'button' }, 'How to install');
      install.addEventListener('click', () => vscode.postMessage({ type: 'install' }));
      const settings = element('button', { type: 'button' }, 'Set ortho.path');
      settings.addEventListener('click', () => vscode.postMessage({ type: 'settings' }));
      node.append(element('br'), install, settings);
    } else if (data.type === 'rendered') {
      // Another model's diagrams never stand in for this one's.
      if (state.folder !== data.folder) views = [];
      state.folder = data.folder;
      save();
      messages.replaceChildren();
      for (const error of data.errors) message(error, 'error');
      for (const note of data.notes) message(note, data.failed ? 'error' : 'info');
      const time = new Date().toLocaleTimeString();
      if (data.views.length > 0) {
        views = data.views;
        requirements = data.requirements ?? { byId: {}, byName: {} };
        showTabs();
        showSelected();
        diagram.classList.remove('stale');
        showActions('current');
        status.textContent = `${data.model}: ${views.length} view${views.length === 1 ? '' : 's'}, ${time}`;
      } else if (views.length > 0) {
        // The model does not load now: the last diagrams stay, faded, until it does.
        diagram.classList.add('stale');
        showActions('stale');
        status.textContent = `${data.model}: not rendered, ${time}`;
      } else {
        diagram.replaceChildren(element('p', { class: 'empty' }, 'No diagrams yet.'));
        showActions('none');
        status.textContent = `${data.model}: not rendered, ${time}`;
      }
    }
  });
})();
