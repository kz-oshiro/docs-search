"""Create the small Windows icon using only Python's standard library."""
import struct
from pathlib import Path

SIZE = 32
pixels = bytearray()
for y in reversed(range(SIZE)):
    for x in range(SIZE):
        color = (18, 110, 114)
        distance = ((x - 13) ** 2 + (y - 13) ** 2) ** 0.5
        handle = 19 <= x <= 27 and 19 <= y <= 27 and abs(x - y) <= 2
        if 7 <= distance <= 9 or handle:
            color = (255, 255, 255)
        r, g, b = color
        pixels.extend((b, g, r, 255))

mask = bytes(4 * SIZE)
bitmap = struct.pack("<IIIHHIIIIII", 40, SIZE, SIZE * 2, 1, 32, 0, len(pixels) + len(mask), 0, 0, 0, 0) + pixels + mask
header = struct.pack("<HHH", 0, 1, 1)
entry = struct.pack("<BBBBHHII", SIZE, SIZE, 0, 0, 1, 32, len(bitmap), len(header) + 16)
destination = Path(__file__).resolve().parent.parent / "src-tauri" / "icons" / "icon.ico"
destination.parent.mkdir(parents=True, exist_ok=True)
destination.write_bytes(header + entry + bitmap)
print(destination)
