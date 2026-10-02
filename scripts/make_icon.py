#!/usr/bin/env python3
"""Generate Cosmogon's original app icon (no external assets, Python stdlib only).

A lit planet with an atmosphere rim and an amber orbital arc on a deep-navy rounded
square. Writes packaging/icons/cosmogon-1024.png and cosmogon.ico (PNG-compressed ICO).
"""
import math, os, struct, zlib

S = 1024
OUT = os.path.join(os.path.dirname(__file__), "..", "packaging", "icons")

def clamp(x, a=0.0, b=1.0):
    return max(a, min(b, x))

def smooth(e0, e1, x):
    t = clamp((x - e0) / (e1 - e0))
    return t * t * (3 - 2 * t)

def hash2(x, y):
    h = (x * 374761393 + y * 668265263) & 0xFFFFFFFF
    h = (h ^ (h >> 13)) * 1274126177 & 0xFFFFFFFF
    return (h ^ (h >> 16)) / 0xFFFFFFFF

def vnoise(x, y):
    xi, yi = math.floor(x), math.floor(y)
    xf, yf = x - xi, y - yi
    u, v = xf * xf * (3 - 2 * xf), yf * yf * (3 - 2 * yf)
    a, b = hash2(xi, yi), hash2(xi + 1, yi)
    c, d = hash2(xi, yi + 1), hash2(xi + 1, yi + 1)
    return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v

def fbm(x, y):
    s, amp, f = 0.0, 0.5, 1.0
    for _ in range(5):
        s += amp * vnoise(x * f, y * f)
        amp *= 0.5
        f *= 2.0
    return s

def render():
    px = bytearray()
    cx, cy, R = S * 0.5, S * 0.52, S * 0.26
    L = (-0.62, -0.48, 0.62)
    ln = math.sqrt(sum(c * c for c in L)); L = tuple(c / ln for c in L)
    for y in range(S):
        px.append(0)
        for x in range(S):
            # Rounded-square mask (macOS-style squircle-ish).
            nx, ny = (x - S / 2) / (S / 2), (y - S / 2) / (S / 2)
            sq = (abs(nx) ** 5 + abs(ny) ** 5) ** 0.2
            alpha = 1.0 - smooth(0.80, 0.805, sq)
            # Background: deep navy gradient with faint stars.
            g = 0.5 + 0.5 * ny
            r, gr, b = 0.004 + 0.008 * (1 - g), 0.008 + 0.012 * (1 - g), 0.02 + 0.03 * (1 - g)
            if hash2(x // 3, y // 3) > 0.9975:
                r, gr, b = r + 0.7, gr + 0.7, b + 0.75
            dx, dy = (x - cx) / R, (y - cy) / R
            d2 = dx * dx + dy * dy
            # Atmosphere halo.
            halo = math.exp(-max(0.0, math.sqrt(d2) - 1.0) * 14.0) * smooth(1.0, 0.98, math.sqrt(d2) - 0.0) if d2 > 1.0 else 0.0
            if d2 > 1.0:
                lit = clamp(0.5 - 0.5 * (dx * L[0] + dy * L[1]) / math.sqrt(d2) * -1.0)
                r += 0.25 * halo * lit; gr += 0.5 * halo * lit; b += 1.0 * halo * lit
            else:
                z = math.sqrt(1 - d2)
                ndl = dx * L[0] + dy * L[1] + z * L[2]
                lam = clamp(ndl)
                n = fbm(dx * 2.6 + 3.1, dy * 2.6 + z * 1.7 + 7.0)
                land = n > 0.52
                if land:
                    c = (0.20 + 0.25 * (n - 0.52) * 4, 0.36 + 0.1 * n, 0.16)
                else:
                    c = (0.03, 0.10 + 0.05 * z, 0.26 + 0.08 * z)
                cl = clamp((fbm(dx * 4 + 11, dy * 7 + 2) - 0.5) * 3.2)
                c = tuple(c[i] * (1 - cl) + 0.92 * cl for i in range(3))
                rim = (1 - z) ** 2.5
                r = c[0] * lam + 0.25 * rim * lam
                gr = c[1] * lam + 0.45 * rim * lam
                b = c[2] * lam + 0.9 * rim * lam
                # City lights on the night side.
                if ndl < -0.05 and land and hash2(x // 6, y // 6) > 0.86:
                    w = smooth(-0.05, -0.35, ndl)
                    r += 1.0 * w; gr += 0.68 * w; b += 0.3 * w
            # Amber orbit arc in front of / behind the planet.
            ex, ey = (x - cx) / (S * 0.37), (y - cy) / (S * 0.11)
            ring = abs(math.sqrt(ex * ex + ey * ey) - 1.0)
            front = y > cy or d2 > 1.0
            if ring < 0.012 and front:
                k = smooth(0.012, 0.0, ring)
                r, gr, b = r + 0.91 * k, gr + 0.69 * k, b + 0.29 * k
            def enc(v):
                v = v / (1 + v * 0.15)
                return int(clamp(v) ** (1 / 2.2) * 255 + 0.5)
            px += bytes((enc(r), enc(gr), enc(b), int(alpha * 255)))
    return bytes(px)

def png(data, w, h):
    def chunk(t, d):
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(data, 9)) + chunk(b"IEND", b"")

def downsample(data, src, dst):
    f = src // dst
    out = bytearray()
    for y in range(dst):
        out.append(0)
        for x in range(dst):
            acc = [0, 0, 0, 0]
            for yy in range(f):
                row = (y * f + yy) * (src * 4 + 1) + 1
                for xx in range(f):
                    i = row + (x * f + xx) * 4
                    for c in range(4):
                        acc[c] += data[i + c]
            out += bytes(v // (f * f) for v in acc)
    return bytes(out)

if __name__ == "__main__":
    os.makedirs(OUT, exist_ok=True)
    big = render()
    open(os.path.join(OUT, "cosmogon-1024.png"), "wb").write(png(big, S, S))
    entries = [(n, png(downsample(big, S, n), n, n)) for n in (256, 64, 48, 32, 16)]
    header = struct.pack("<HHH", 0, 1, len(entries))
    offset = 6 + 16 * len(entries)
    dir_, blobs = b"", b""
    for n, blob in entries:
        dir_ += struct.pack("<BBBBHHII", n % 256, n % 256, 0, 0, 1, 32, len(blob), offset + len(blobs))
        blobs += blob
    open(os.path.join(OUT, "cosmogon.ico"), "wb").write(header + dir_ + blobs)
    print("wrote", OUT)
