#!/usr/bin/env python3
"""README 用のデモ GIF とギャラリー画像を作る。

魔法陣のコマは、release ビルドの maho が Kitty graphics protocol で端末へ送った
画像をそのまま抜き出して使う。ターミナルの枠・プロンプト・コマンドの出力だけを
ImageMagick で描き足す。実際のコマンドは走らせず、何もしない偽物を PATH に置く。

必要なもの: cargo, ImageMagick (magick), script(1)
使い方:     python3 scripts/demo.py   # assets/demo.gif と assets/gallery.jpg を書き出す
フォント:   既定は macOS の Monaco。MAHO_DEMO_FONT で差し替えられる。
"""

import base64
import os
import re
import shlex
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets"
MAHO = ROOT / "target" / "release" / "maho"
FONT = os.environ.get("MAHO_DEMO_FONT", "/System/Library/Fonts/Monaco.ttf")

# デモで打つコマンドと、その後に出す（それらしい）出力
DEMOS = [
    ("git status", [
        "On branch main",
        "Your branch is up to date with 'origin/main'.",
        "",
        "nothing to commit, working tree clean",
    ]),
    ("cargo build --release", [
        "   Compiling maho v0.1.0 (~/maho)",
        "    Finished `release` profile [optimized] target(s) in 4.21s",
    ]),
    ("npm test", [
        "> app@1.0.0 test",
        "> vitest run",
        "",
        " Test Files  3 passed (3)",
    ]),
]

# ギャラリーに並べるコマンド。1 文字違いのものを隣に置く
GALLERY = [
    "git status", "git commit", "git commit -m 'fix'", "git push",
    "ls", "ls -la", "cargo build", "docker compose up",
]

W, H = 720, 720
CIRCLE = 512
CIRCLE_X, CIRCLE_Y = (W - CIRCLE) // 2, 76
LINE_H = 24
BG, BAR = "#11111b", "#1e1e2e"


def capture(spell: str, stub_dir: Path, work: Path) -> list[Path]:
    """maho に spell を唱えさせ、端末へ送られたコマを PNG にして返す。
    spell は引用符付きの引数（-m 'fix'）もシェルと同じように割って渡す。"""
    out = work / "tty.out"
    env = {
        "PATH": f"{stub_dir}:/usr/bin:/bin",
        "TERM_PROGRAM": "ghostty",
        "HOME": os.environ.get("HOME", "/tmp"),
    }
    subprocess.run(
        ["script", "-q", str(out), str(MAHO), *shlex.split(spell)],
        env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, check=True,
    )
    data = out.read_bytes()
    frames, chunk = [], b""
    for ctrl, payload in re.findall(rb"\x1b_G([^;]*);([^\x1b]*)\x1b\\", data):
        chunk += payload
        if b"m=0" in ctrl:
            frames.append(base64.b64decode(chunk))
            chunk = b""
    paths = []
    for i, png in enumerate(frames):
        p = work / f"{slug(spell)}-{i:02d}.png"
        p.write_bytes(png)
        paths.append(p)
    if not paths:
        sys.exit(f"maho から画像を受け取れませんでした: {spell}")
    return paths


def slug(s: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", s.lower()).strip("-")


def terminal_frame(dest: Path, typed: str, circle: Path | None, output: list[str]):
    """ターミナル 1 コマを描く。"""
    cmd = [
        "magick", "-size", f"{W}x{H}", f"xc:{BG}",
        "-fill", BAR, "-draw", f"rectangle 0,0 {W},32",
        "-fill", "#f38ba8", "-draw", "circle 20,16 26,16",
        "-fill", "#f9e2af", "-draw", "circle 42,16 48,16",
        "-fill", "#a6e3a1", "-draw", "circle 64,16 70,16",
        "-font", FONT, "-pointsize", "14", "-fill", "#6c7086",
        "-annotate", "+330+21", "maho",
        "-pointsize", "17",
        "-fill", "#a6e3a1", "-annotate", "+20+60", "$",
        "-fill", "#cdd6f4", "-annotate", "+40+60", typed,
    ]
    if circle is not None:
        cmd += [str(circle), "-geometry", f"+{CIRCLE_X}+{CIRCLE_Y}", "-composite"]
    y = CIRCLE_Y + CIRCLE + 28
    for line in output:
        if line:
            cmd += ["-pointsize", "15", "-fill", "#cdd6f4", "-annotate", f"+20+{y}", line]
        y += LINE_H
    cmd.append(str(dest))
    subprocess.run(cmd, check=True)


def build_demo(stub_dir: Path, work: Path):
    frames: list[tuple[Path, int]] = []  # (画像, 1/100 秒単位の表示時間)
    n = 0

    def add(typed, circle, output, delay):
        nonlocal n
        p = work / f"demo-{n:04d}.png"
        terminal_frame(p, typed, circle, output)
        frames.append((p, delay))
        n += 1

    for spell, output in DEMOS:
        circles = capture(spell, stub_dir, work)
        line = f"maho {spell}"
        for k in range(0, len(line) + 1, 2):
            add(line[:k], None, [], 5)
        add(line, None, [], 35)
        # maho と同じく 16 コマをおよそ 45ms 間隔で
        for c in circles:
            add(line, c, [], 5)
        add(line, circles[-1], [], 20)
        add(line, circles[-1], output, 230)

    args = ["magick", "-loop", "0"]
    for p, delay in frames:
        args += ["-delay", str(delay), str(p)]
    args += ["-layers", "Optimize", str(ASSETS / "demo.gif")]
    subprocess.run(args, check=True)


def build_gallery(stub_dir: Path, work: Path):
    tiles = []
    for spell in GALLERY:
        last = capture(spell, stub_dir, work)[-1]
        tiles += ["-label", spell, str(last)]
    subprocess.run([
        "magick", "montage", *tiles,
        "-tile", "4x2", "-geometry", "320x320+8+8",
        "-font", FONT, "-pointsize", "18",
        "-fill", "#cdd6f4", "-background", BG, "-quality", "85",
        str(ASSETS / "gallery.jpg"),
    ], check=True)


def main():
    subprocess.run(["cargo", "build", "--release", "-q"], cwd=ROOT, check=True)
    ASSETS.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        stub_dir = work / "stub"
        stub_dir.mkdir()
        # 呪文に出てくるコマンドは、何もしない偽物にすり替えておく
        for name in {s.split()[0] for s, _ in DEMOS} | {s.split()[0] for s in GALLERY}:
            stub = stub_dir / name
            stub.write_text("#!/bin/sh\n")
            stub.chmod(0o755)
        build_demo(stub_dir, work)
        build_gallery(stub_dir, work)
    for f in ("demo.gif", "gallery.jpg"):
        size = (ASSETS / f).stat().st_size
        print(f"assets/{f}  {size / 1024:.0f} KiB")


if __name__ == "__main__":
    main()
