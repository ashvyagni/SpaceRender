#!/usr/bin/env python3
"""Convert NOAA ETOPO5 (5 arc-minute global relief, public domain) into the compact grid
embedded in the simulation: crates/cosmogon_sim/data/earth_elevation.bin.

Input:  tools/data/ETOPO5.DOS  — 4320×2160 little-endian int16 metres, row 0 = 90°N,
        column 0 = 0°E (download: https://www.ngdc.noaa.gov/mgg/global/relief/ETOPO5/TOPO/ETOPO5/ETOPO5.DOS)
Output: 2048×1024 little-endian int16 metres, same orientation, area-averaged.
"""
import array, os, sys

SRC_W, SRC_H = 4320, 2160
DST_W, DST_H = 2048, 1024
root = os.path.join(os.path.dirname(__file__), "..")
src = array.array("h")
with open(os.path.join(root, "tools/data/ETOPO5.DOS"), "rb") as f:
    src.frombytes(f.read())
if sys.byteorder != "little":
    src.byteswap()
assert len(src) == SRC_W * SRC_H

# Box filter: each destination cell averages the source cells it covers (≈2.1×2.1).
out = array.array("h", [0]) * (DST_W * DST_H)
for y in range(DST_H):
    y0 = y * SRC_H // DST_H
    y1 = max(y0 + 1, (y + 1) * SRC_H // DST_H)
    for x in range(DST_W):
        x0 = x * SRC_W // DST_W
        x1 = max(x0 + 1, (x + 1) * SRC_W // DST_W)
        s = 0
        n = 0
        for yy in range(y0, y1):
            row = yy * SRC_W
            for xx in range(x0, x1):
                s += src[row + xx]
                n += 1
        out[y * DST_W + x] = int(round(s / n))
if sys.byteorder != "little":
    out.byteswap()
dst = os.path.join(root, "crates/cosmogon_sim/data/earth_elevation.bin")
with open(dst, "wb") as f:
    f.write(out.tobytes())
print("wrote", dst, os.path.getsize(dst), "bytes")
