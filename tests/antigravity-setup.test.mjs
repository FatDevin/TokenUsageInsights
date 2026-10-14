import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import i18n from '../static/i18n.js';

const app = readFileSync(new URL('../static/app.js', import.meta.url), 'utf8');
const html = readFileSync(new URL('../static/index.html', import.meta.url), 'utf8');

test('Antigravity native guide and Copilot collector guide remain separate', () => {
  const bodies = new Map([...html.matchAll(/id="(setup-body-[^"]+)"/g)].map(([, id]) => [id, {
    style: { display: 'none', removeProperty(key) { delete this[key]; } },
  }]));
  const start = app.indexOf('function setSetupModalBody(');
  const end = app.indexOf('\nfunction updateLanguageUI(', start);
  const context = vm.createContext({ document: { getElementById: id => bodies.get(id) } });
  vm.runInContext(app.slice(start, end), context);
  for (const [assistant, selected] of [['antigravity', 'setup-body-antigravity'], ['copilot', 'setup-body-statusline']]) {
    context.setSetupModalBody(assistant);
    for (const [id, body] of bodies) {
      assert.equal(body.style.display, id === selected ? undefined : 'none', `${assistant}: ${id}`);
    }
  }
});

test('native guide translations exist in every locale and preserve Copilot setup', () => {
  const nativeGuide = html.split('id="setup-body-antigravity"')[1].split('<!-- Status Line')[0];
  const keys = [...nativeGuide.matchAll(/data-i18n="([^"]+)"/g)].map(([, key]) => key);
  assert.match(nativeGuide, /transcript_full\.jsonl/);
  assert.match(nativeGuide, /conversations\//);
  assert.doesNotMatch(nativeGuide, /settings\.json|statusline-token/);
  for (const [locale, translations] of Object.entries(i18n)) {
    for (const key of keys) assert.ok(translations[key], `${locale}: ${key}`);
    assert.match(translations.copilot_setup_modal_intro, /Copilot CLI/);
    assert.match(translations.copilot_setup_modal_intro, /statusline-token/);
    assert.doesNotMatch(translations.antigravity_setup_modal_intro, /statusline-token/);
    assert.doesNotMatch(translations.antigravity_empty_desc, /settings\.json|statusline-token/);
  }
});
