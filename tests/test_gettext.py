"""Regression coverage for the Debian gettext compatibility pass."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('prepare_gettext', Path(__file__).resolve().parents[1] / 'scripts/prepare-gettext.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class GettextTests(unittest.TestCase):
    def test_lifetimes_do_not_hide_messages_or_change_lines(self):
        source = 'fn lookup<\'a>(value: &\'a str) -> &\'static str {\n gettext("Asena\'s panel");\n}\n'
        result = module.prepare(source)
        self.assertNotIn("'a", result)
        self.assertNotIn("'static", result)
        self.assertIn('gettext("Asena\'s panel")', result)
        self.assertEqual(result.count('\n'), source.count('\n'))

    def test_character_literals_and_comments_are_preserved(self):
        source = '''let c = 'İ'; let quote = '\\''; // 'lifetime
/* 'static */ gettext("Close");'''
        self.assertEqual(module.prepare(source), source)
