#!/usr/bin/env python3
"""Rebuild the bundled coverage grid; dev-only dependencies: resvg 0.45.1, Pillow.

Usage: python3 scripts/build_logo_mask.py --resvg /path/to/resvg
The runtime uses the generated TOML and needs neither tool.
"""
import argparse
from pathlib import Path
import subprocess
import tempfile

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
MARKER = "\n# Detailed coverage: Simple Icons"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--resvg", default="resvg")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory() as directory:
        image = Path(directory) / "mask.png"
        subprocess.run([
            args.resvg, "--width", "128", "--height", "128",
            str(ROOT / "assets/logos/apple.svg"), str(image),
        ], check=True)
        with Image.open(image) as raster:
            alpha = raster.getchannel("A")
            rows = ["".join(format(round(alpha.getpixel((x, y)) / 17), "x")
                            for x in range(128)) for y in range(128)]
    asset = ROOT / "assets/logos/macos.toml"
    original = asset.read_text().split(MARKER)[0].rstrip()
    asset.write_text(original + MARKER + " Apple (CC0), see THIRD_PARTY_NOTICES.\n"
                     + "# Square pixels; 0 = transparent, f = opaque. Generated, do not hand-edit.\n"
                     + "mask = '''\n" + "\n".join(rows) + "\n'''\n")


if __name__ == "__main__":
    main()
