#!/usr/bin/env python3
"""Embed the maintained distinctive-algorithm chapter in the reader document."""
from pathlib import Path

root = Path(__file__).resolve().parent
target = root / 'DESIGN_KO.md'
start = '<!-- DISTINCTIVE_CHAPTER_BEGIN -->'
end = '<!-- DISTINCTIVE_CHAPTER_END -->'
text = target.read_text()
chapter = (root / 'DISTINCTIVE_ALGORITHMS_KO.md').read_text().rstrip()
block = f'{start}\n\n{chapter}\n\n{end}\n\n'
if start in text:
    before, rest = text.split(start, 1)
    _, after = rest.split(end, 1)
    text = before + block.rstrip() + after
else:
    marker = '# 명시적 Dormand–Prince 5(4) 방법'
    if text.count(marker) != 1:
        raise SystemExit('Expected unique insertion heading')
    text = text.replace(marker, block + marker)
target.write_text(text)
