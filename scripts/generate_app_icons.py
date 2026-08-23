#!/usr/bin/env python3
"""Generate the deterministic V1 Talking Moose application icon set.

The artwork is original project pixel/vector-like geometry rendered with the Python
standard library only. Release builds regenerate every binary icon from this source,
so no opaque placeholder icon blob is required in Git.
"""
from __future__ import annotations

import struct
import zlib
from pathlib import Path

OUTPUT = Path(__file__).resolve().parents[1] / "src-tauri" / "icons"

Color = tuple[int, int, int, int]


def canvas(size: int, color: Color) -> bytearray:
    return bytearray(color * (size * size))


def paint_pixel(data: bytearray, size: int, x: int, y: int, color: Color) -> None:
    if 0 <= x < size and 0 <= y < size:
        offset = (y * size + x) * 4
        data[offset : offset + 4] = bytes(color)


def rectangle(data: bytearray, size: int, box: tuple[float, float, float, float], color: Color) -> None:
    left, top, right, bottom = (round(value * size) for value in box)
    for y in range(max(0, top), min(size, bottom)):
        start = (y * size + max(0, left)) * 4
        end = (y * size + min(size, right)) * 4
        data[start:end] = bytes(color) * max(0, min(size, right) - max(0, left))


def ellipse(data: bytearray, size: int, box: tuple[float, float, float, float], color: Color) -> None:
    left, top, right, bottom = box
    cx = (left + right) / 2
    cy = (top + bottom) / 2
    rx = (right - left) / 2
    ry = (bottom - top) / 2
    x0, y0, x1, y1 = (round(value * size) for value in box)
    for y in range(max(0, y0), min(size, y1 + 1)):
        ny = (y / size - cy) / ry
        if abs(ny) > 1:
            continue
        span = rx * (1 - ny * ny) ** 0.5
        xa = max(0, round((cx - span) * size))
        xb = min(size - 1, round((cx + span) * size))
        for x in range(xa, xb + 1):
            paint_pixel(data, size, x, y, color)


def disk(data: bytearray, size: int, cx: float, cy: float, radius: float, color: Color) -> None:
    ellipse(data, size, (cx - radius, cy - radius, cx + radius, cy + radius), color)


def thick_line(
    data: bytearray,
    size: int,
    start: tuple[float, float],
    end: tuple[float, float],
    width: float,
    color: Color,
) -> None:
    x0, y0 = start
    x1, y1 = end
    steps = max(1, round(max(abs(x1 - x0), abs(y1 - y0)) * size * 1.5))
    radius = width / 2
    for index in range(steps + 1):
        t = index / steps
        disk(data, size, x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, radius, color)


def draw_icon(size: int) -> bytes:
    outer: Color = (222, 217, 207, 255)
    cream: Color = (239, 233, 222, 255)
    border: Color = (30, 25, 20, 255)
    antler: Color = (166, 112, 66, 255)
    head: Color = (111, 75, 48, 255)
    inner_ear: Color = (190, 140, 94, 255)
    muzzle: Color = (201, 161, 112, 255)
    white: Color = (250, 248, 242, 255)

    data = canvas(size, outer)
    rectangle(data, size, (0.04, 0.04, 0.96, 0.96), border)
    rectangle(data, size, (0.07, 0.07, 0.93, 0.93), cream)

    left_antler = [
        ((0.36, 0.40), (0.28, 0.32)),
        ((0.28, 0.32), (0.22, 0.22)),
        ((0.29, 0.30), (0.30, 0.17)),
        ((0.34, 0.32), (0.36, 0.20)),
        ((0.36, 0.34), (0.42, 0.27)),
    ]
    for start, end in left_antler:
        thick_line(data, size, start, end, 0.075, border)
        thick_line(data, size, start, end, 0.045, antler)
        mirrored_start = (1 - start[0], start[1])
        mirrored_end = (1 - end[0], end[1])
        thick_line(data, size, mirrored_start, mirrored_end, 0.075, border)
        thick_line(data, size, mirrored_start, mirrored_end, 0.045, antler)

    ellipse(data, size, (0.22, 0.36, 0.42, 0.51), border)
    ellipse(data, size, (0.245, 0.385, 0.40, 0.49), head)
    ellipse(data, size, (0.58, 0.36, 0.78, 0.51), border)
    ellipse(data, size, (0.60, 0.385, 0.755, 0.49), head)
    ellipse(data, size, (0.28, 0.405, 0.37, 0.465), inner_ear)
    ellipse(data, size, (0.63, 0.405, 0.72, 0.465), inner_ear)

    ellipse(data, size, (0.29, 0.30, 0.71, 0.81), border)
    ellipse(data, size, (0.315, 0.325, 0.685, 0.785), head)

    ellipse(data, size, (0.36, 0.405, 0.50, 0.55), border)
    ellipse(data, size, (0.378, 0.423, 0.482, 0.532), white)
    ellipse(data, size, (0.50, 0.395, 0.64, 0.54), border)
    ellipse(data, size, (0.518, 0.413, 0.622, 0.522), white)
    ellipse(data, size, (0.435, 0.462, 0.475, 0.515), border)
    ellipse(data, size, (0.535, 0.445, 0.575, 0.498), border)

    ellipse(data, size, (0.33, 0.54, 0.67, 0.79), border)
    ellipse(data, size, (0.35, 0.565, 0.65, 0.765), muzzle)
    ellipse(data, size, (0.405, 0.61, 0.46, 0.66), border)
    ellipse(data, size, (0.54, 0.61, 0.595, 0.66), border)
    thick_line(data, size, (0.41, 0.70), (0.49, 0.72), 0.016, border)
    thick_line(data, size, (0.49, 0.72), (0.58, 0.695), 0.016, border)

    return png(data, size, size)


def png(rgba: bytes | bytearray, width: int, height: int) -> bytes:
    signature = b"\x89PNG\r\n\x1a\n"

    def chunk(kind: bytes, payload: bytes) -> bytes:
        body = kind + payload
        return struct.pack(">I", len(payload)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    rows = b"".join(
        b"\x00" + bytes(rgba[y * width * 4 : (y + 1) * width * 4]) for y in range(height)
    )
    return (
        signature
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(rows, 9))
        + chunk(b"IEND", b"")
    )


def write_icns(source_png: bytes) -> None:
    chunk = b"ic10" + struct.pack(">I", 8 + len(source_png)) + source_png
    (OUTPUT / "icon.icns").write_bytes(b"icns" + struct.pack(">I", 8 + len(chunk)) + chunk)


def write_ico(images: list[tuple[int, bytes]]) -> None:
    header = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    entries: list[bytes] = []
    payload: list[bytes] = []
    for size, data in images:
        encoded_size = 0 if size == 256 else size
        entries.append(struct.pack("<BBBBHHII", encoded_size, encoded_size, 0, 0, 1, 32, len(data), offset))
        payload.append(data)
        offset += len(data)
    (OUTPUT / "icon.ico").write_bytes(header + b"".join(entries) + b"".join(payload))


def main() -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    generated = {size: draw_icon(size) for size in (32, 128, 256, 1024)}
    (OUTPUT / "32x32.png").write_bytes(generated[32])
    (OUTPUT / "128x128.png").write_bytes(generated[128])
    (OUTPUT / "128x128@2x.png").write_bytes(generated[256])
    (OUTPUT / "icon.png").write_bytes(generated[1024])
    write_icns(generated[1024])
    write_ico([(32, generated[32]), (128, generated[128]), (256, generated[256])])


if __name__ == "__main__":
    main()
