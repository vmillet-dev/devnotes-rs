"""SSIM between the egui and the Chromium PNG of each sample, after aligning both on their ink (best of ±2 px).

Grey levels, the ink box of each image (pixels brighter than the background), both crops padded
to the same size, SSIM with a 7×7 uniform window (the skimage default). Also the ink width ratio,
which is what a different weight or advance shows first. Usage: python3 -I ssim.py ../out/s2
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image

def grey(path):
    return np.asarray(Image.open(path).convert('L'), dtype=np.float64)

def ink(img):
    mask = img > img[0, 0] + 24
    ys, xs = np.nonzero(mask)
    if len(xs) == 0:
        return img[:1, :1]
    return img[ys.min():ys.max() + 1, xs.min():xs.max() + 1]

def pad(a, h, w, value):
    out = np.full((h, w), value)
    out[:a.shape[0], :a.shape[1]] = a
    return out

def window_mean(a, k=7):
    c = np.cumsum(np.cumsum(np.pad(a, ((1, 0), (1, 0))), 0), 1)
    return (c[k:, k:] - c[:-k, k:] - c[k:, :-k] + c[:-k, :-k]) / (k * k)

def ssim(a, b):
    c1, c2 = (0.01 * 255) ** 2, (0.03 * 255) ** 2
    ma, mb = window_mean(a), window_mean(b)
    va = window_mean(a * a) - ma ** 2
    vb = window_mean(b * b) - mb ** 2
    cov = window_mean(a * b) - ma * mb
    s = ((2 * ma * mb + c1) * (2 * cov + c2)) / ((ma ** 2 + mb ** 2 + c1) * (va + vb + c2))
    return float(s.mean())

root = Path(sys.argv[1])
print(f"{'sample':<16}" + ''.join(f"{'SSIM ' + s + ' %':>12}{'width':>8}" for s in ('100', '125', '150')))
for sample in sorted((root / 'egui' / '100').glob('*.png')):
    row = f'{sample.stem:<16}'
    for scale in ('100', '125', '150'):
        e = grey(root / 'egui' / scale / sample.name)
        c = grey(root / 'chromium' / scale / sample.name)
        background = e[0, 0]
        ie, ic = ink(e), ink(c)
        h, w = max(ie.shape[0], ic.shape[0]), max(ie.shape[1], ic.shape[1])
        a = pad(ie, h + 4, w + 4, background)
        best = max(ssim(np.roll(np.roll(a, dy, 0), dx, 1), pad(ic, h + 4, w + 4, background))
                   for dy in range(-2, 3) for dx in range(-2, 3))
        row += f'{best:12.3f}{ie.shape[1] / ic.shape[1]:8.3f}'
    print(row)
