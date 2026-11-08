// @ts-check
// The preview panel's page: a tab for each view ortho drew, the chosen view's diagram,
// and what ortho reported. The extension sends each render as a message.
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

  /**
   * The save buttons: hidden with no diagrams, and disabled while the model does not
   * load, since the diagrams shown then no longer match it.
   * @param {'none' | 'current' | 'stale'} state
   */
  function showActions(state) {
    actions.hidden = state === 'none';
    const why = state === 'stale' ? 'The model does not load now: fix its problems to save its diagrams' : '';
    saveOne.disabled = saveAll.disabled = state === 'stale';
    saveOne.title = why || 'Save the view shown as an SVG file';
    saveAll.title = why || 'Save every view as SVG, into a folder';
  }

  /** @type {{ folder?: string, selected?: string, actualSize?: boolean }} */
  const state = vscode.getState() || {};
  /** @type {{ file: string, title: string, heading: string, summary: string, svg: string }[]} */
  let views = [];

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
    const view = views.find((v) => v.title === state.selected) || views[0];
    for (const button of tabs.querySelectorAll('button')) {
      button.setAttribute('aria-selected', String(button.textContent === view?.title));
    }
    diagram.replaceChildren();
    diagram.classList.toggle('actual-size', Boolean(state.actualSize));
    if (!view) return;
    // The extension saves the view on screen.
    vscode.postMessage({ type: 'showing', file: view.file });
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
