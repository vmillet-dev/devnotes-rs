"""Writes fonts/*.ttf from the woff2 files of @fontsource-variable (the ones the app ships today).

Usage: python3 -I fonts.py <node_modules/@fontsource-variable>  (needs fonttools and brotli)
"""
import sys
from pathlib import Path

from fontTools.ttLib import TTFont

source = Path(sys.argv[1])
out = Path(__file__).parent / 'fonts'
out.mkdir(exist_ok=True)
for woff2, ttf in [
    ('inter/files/inter-latin-wght-normal.woff2', 'InterVariable-latin.ttf'),
    ('inter/files/inter-latin-ext-wght-normal.woff2', 'InterVariable-latin-ext.ttf'),
    ('jetbrains-mono/files/jetbrains-mono-latin-wght-normal.woff2', 'JetBrainsMonoVariable-latin.ttf'),
]:
    font = TTFont(source / woff2)
    font.flavor = None
    font.save(out / ttf)
