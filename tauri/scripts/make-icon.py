"""Rasterize the simple approved SVG mark into Windows icons using only stdlib."""

import math
import struct
import xml.etree.ElementTree as ET
import zlib
from pathlib import Path

ICON_DIRECTORY = Path(__file__).resolve().parent.parent / "src-tauri" / "icons"
SIZES = (16, 20, 24, 32, 48, 64, 128, 256)
SAMPLES = 4


def color(value):
    if value in (None, "none"):
        return None
    return tuple(int(value[index:index + 2], 16) for index in (1, 3, 5))


def parse_shapes(source):
    root = ET.parse(source).getroot()
    if root.attrib.get("viewBox") != "0 0 128 128":
        raise ValueError("The icon must use viewBox 0 0 128 128")
    shapes = []
    for element in root:
        kind = element.tag.rsplit("}", 1)[-1]
        if kind == "title":
            continue
        if kind not in ("rect", "circle", "line", "polygon"):
            raise ValueError(f"Unsupported icon element: {kind}")
        shape = {"kind": kind, "fill": color(element.get("fill")), "stroke": color(element.get("stroke"))}
        for key, value in element.attrib.items():
            if key in ("fill", "stroke"):
                continue
            if key == "points":
                shape[key] = [tuple(map(float, point.split(","))) for point in value.split()]
            elif key == "stroke-linecap":
                if value != "round":
                    raise ValueError("Only round line caps are supported")
            else:
                shape[key] = float(value)
        if kind == "line" and element.get("stroke-linecap") != "round":
            raise ValueError("Icon lines must use round line caps")
        shapes.append(shape)
    return shapes


def paint(shape, x, y):
    kind = shape["kind"]
    if kind == "rect":
        left, top, width, height = (shape[key] for key in ("x", "y", "width", "height"))
        if not (left <= x <= left + width and top <= y <= top + height):
            return None
        radius = min(shape.get("rx", 0), width / 2, height / 2)
        center_x = min(max(x, left + radius), left + width - radius)
        center_y = min(max(y, top + radius), top + height - radius)
        return shape["fill"] if (x - center_x) ** 2 + (y - center_y) ** 2 <= radius ** 2 else None
    if kind == "circle":
        distance = math.hypot(x - shape["cx"], y - shape["cy"])
        if shape["stroke"] is not None and abs(distance - shape["r"]) <= shape.get("stroke-width", 1) / 2:
            return shape["stroke"]
        return shape["fill"] if distance <= shape["r"] else None
    if kind == "line":
        dx, dy = shape["x2"] - shape["x1"], shape["y2"] - shape["y1"]
        fraction = ((x - shape["x1"]) * dx + (y - shape["y1"]) * dy) / (dx * dx + dy * dy)
        fraction = min(1, max(0, fraction))
        distance = math.hypot(x - shape["x1"] - fraction * dx, y - shape["y1"] - fraction * dy)
        return shape["stroke"] if distance <= shape.get("stroke-width", 1) / 2 else None
    inside = False
    points = shape["points"]
    previous = points[-1]
    for point in points:
        x1, y1 = previous
        x2, y2 = point
        if (y1 > y) != (y2 > y) and x < (x2 - x1) * (y - y1) / (y2 - y1) + x1:
            inside = not inside
        previous = point
    return shape["fill"] if inside else None


def rasterize(shapes, size):
    pixels = bytearray()
    scale = 128 / size
    for y in range(size):
        for x in range(size):
            totals = [0, 0, 0]
            covered = 0
            for sample_y in range(SAMPLES):
                for sample_x in range(SAMPLES):
                    point_x = (x + (sample_x + 0.5) / SAMPLES) * scale
                    point_y = (y + (sample_y + 0.5) / SAMPLES) * scale
                    pixel = None
                    for shape in shapes:
                        painted = paint(shape, point_x, point_y)
                        if painted is not None:
                            pixel = painted
                    if pixel is not None:
                        covered += 1
                        for channel in range(3):
                            totals[channel] += pixel[channel]
            if covered:
                pixels.extend(round(total / covered) for total in totals)
                pixels.append(round(255 * covered / (SAMPLES * SAMPLES)))
            else:
                pixels.extend((0, 0, 0, 0))
    return bytes(pixels)


def bitmap(size, rgba):
    pixels = bytearray()
    mask = bytearray()
    mask_stride = ((size + 31) // 32) * 4
    for y in reversed(range(size)):
        mask_row = bytearray(mask_stride)
        for x in range(size):
            offset = (y * size + x) * 4
            r, g, b, alpha = rgba[offset:offset + 4]
            pixels.extend((b, g, r, alpha))
            if alpha == 0:
                mask_row[x // 8] |= 1 << (7 - x % 8)
        mask.extend(mask_row)
    header = struct.pack("<IIIHHIIIIII", 40, size, size * 2, 1, 32, 0, len(pixels), 0, 0, 0, 0)
    return header + pixels + mask


def png(size, rgba):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    rows = b"".join(b"\0" + rgba[y * size * 4:(y + 1) * size * 4] for y in range(size))
    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b"")


def main():
    shapes = parse_shapes(ICON_DIRECTORY / "icon.svg")
    directory = bytearray(struct.pack("<HHH", 0, 1, len(SIZES)))
    images = bytearray()
    offset = 6 + 16 * len(SIZES)
    for size in SIZES:
        rgba = rasterize(shapes, size)
        data = bitmap(size, rgba)
        dimension = 0 if size == 256 else size
        directory.extend(struct.pack("<BBBBHHII", dimension, dimension, 0, 0, 1, 32, len(data), offset))
        images.extend(data)
        offset += len(data)
        if size == 256:
            (ICON_DIRECTORY / "icon.png").write_bytes(png(size, rgba))
    (ICON_DIRECTORY / "icon.ico").write_bytes(directory + images)
    print(f"Created icon.ico ({', '.join(map(str, SIZES))} px) and icon.png (256 px)")


if __name__ == "__main__":
    main()
