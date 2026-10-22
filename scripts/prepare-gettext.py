"""Hide Rust lifetimes from Debian's C gettext scanner, preserving line numbers."""
from pathlib import Path
import re
import sys

TOKENS = re.compile(
    r'"(?:\\.|[^"\\])*"|//[^\n]*|/\*[\s\S]*?\*/|'
    r"'(?:\\.|[^'\\])'|'[A-Za-z_][A-Za-z0-9_]*"
)


def prepare(source):
    def replace(match):
        token = match.group()
        if token.startswith("'") and not token.endswith("'"):
            return " " + token[1:]
        return token
    return TOKENS.sub(replace, source)


def main():
    root = Path(__file__).resolve().parents[1]
    destination = Path(sys.argv[1])
    for name in (root / 'po/POTFILES').read_text(encoding='utf-8').splitlines():
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(prepare((root / name).read_text(encoding='utf-8')),
                          encoding='utf-8', newline='\n')


if __name__ == '__main__':
    main()
