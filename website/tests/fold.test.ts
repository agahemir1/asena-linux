// The cases of crates/asena-core/src/text.rs, so the site's demo searches the way Asena does.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fold, naiveKey } from '../src/lib/fold.ts';

test('turkish letters and cases', () => {
  for (const [input, expected] of [
    ['İSTANBUL', 'istanbul'],
    ['Işık', 'isik'],
    ['ÇAĞRI', 'cagri'],
    ['İIıi', 'iiii'],
    ['ÇçĞğÖöŞşÜü', 'ccggoossuu'],
  ]) {
    assert.equal(fold(input), expected);
  }
  assert.equal(fold('Ayarlar'), fold('AYARLAR'));
});

test('canonical equivalence and accents', () => {
  assert.equal(fold('İŞIK'), 'isik');
  assert.equal(fold('École Ångström café'), 'ecole angstrom cafe');
  assert.equal(fold('ş'), fold('ş'));
});

test('preserves spacing, symbols and other scripts', () => {
  assert.equal(fold(''), '');
  assert.equal(fold('  A\tB\n42 + 🐺 中文'), '  a\tb\n42 + 🐺 中文');
  assert.equal(fold('Москва'), 'москва');
  for (const input of ['İstanbul', 'Çağrı', 'café', '中文']) {
    assert.equal(fold(fold(input)), fold(input));
  }
});

test('the plain comparison misses what fold finds', () => {
  assert.ok(!naiveKey('İndirilenler').includes(naiveKey('indir')));
  assert.ok(fold('İndirilenler').includes(fold('indir')));
  assert.ok(!naiveKey('Oynatıcı').includes(naiveKey('oynatici')));
  assert.ok(fold('Oynatıcı').includes(fold('oynatici')));
});
