#!/usr/bin/env python3
"""README 用のデモ GIF とギャラリー画像を作る。

魔法陣のコマは、release ビルドの mahojin が Kitty graphics protocol で端末へ送った
画像をそのまま抜き出して使う。ターミナルの枠・プロンプト・コマンドの出力だけを
ImageMagick で描き足す。実際のコマンドは走らせず、何もしない偽物を PATH に置く。

必要なもの: cargo, ImageMagick (magick), script(1)
使い方:     python3 scripts/demo.py   # assets/demo.gif と assets/gallery.jpg を書き出す
フォント:   既定は macOS の Menlo（✦ の字を持っている）。MAHOJIN_DEMO_FONT で差し替えられる。
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
MAHOJIN = ROOT / "target" / "release" / "mahojin"
FONT = os.environ.get("MAHOJIN_DEMO_FONT", "/System/Library/Fonts/Menlo.ttc")
# 暦の兆しが何も無い日時
ORDINARY_DAY = "2026-09-24 12:00"

# デモで打つコマンドと、その後に出す（それらしい）出力。
# 魔法の格が 小 → 中 → 大 → 極大 と上がっていく順に並べる
DEMOS = [
    ("npm test", [
        "> app@1.0.0 test",
        "> vitest run",
        "",
        " Test Files  3 passed (3)",
    ]),
    ("ls -la", [
        "drwxr-xr-x  8 you  staff   256 Sep 24 11:02 .",
        "-rw-r--r--  1 you  staff  2195 Sep 24 11:02 Cargo.toml",
        "drwxr-xr-x  6 you  staff   192 Sep 24 11:02 src",
    ]),
    ("git status", [
        "On branch main",
        "nothing to commit, working tree clean",
    ]),
    ("cargo run", [
        "   Compiling mahojin v0.1.0 (~/mahojin)",
        "    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.02s",
        "     Running `target/debug/mahojin`",
    ]),
]

# ギャラリーに並べるコマンド。似たものを隣に置き、4 つの格がそろうようにする
GALLERY = [
    "git status", "git commit", "make", "git push",
    "ls", "ls -la", "cargo build", "cargo run",
]

W, H = 720, 820
# 端末の 1 行ぶんを何ピクセルで描くか。36 行の極大魔法で 576px になる
ROW_PX = 16
CIRCLE_Y = 76
LINE_H = 24
BG, BAR = "#11111b", "#1e1e2e"


def capture(spell: str, stub_dir: Path, work: Path) -> tuple[list[Path], int, str]:
    """mahojin に spell を唱えさせ、端末へ送られたコマの PNG と、画像の高さ（行数）と、
    画像の下に出た 1 行（大魔法以上で出る名乗り）を返す。
    spell は引用符付きの引数（-m 'fix'）もシェルと同じように割って渡す。"""
    out = work / "tty.out"
    env = {
        "PATH": f"{stub_dir}:/usr/bin:/bin",
        "TERM_PROGRAM": "ghostty",
        "HOME": os.environ.get("HOME", "/tmp"),
        # 名乗りは英語で出させる（デモのフォントに日本語が無いことがある）
        "MAHOJIN_LOCALE": "en",
        # デモで唱えた呪文を図鑑に残さない
        "MAHOJIN_GRIMOIRE": "off",
        # 暦の兆しで色が変わらないよう、何でもない日にする
        "MAHOJIN_NOW": ORDINARY_DAY,
    }
    subprocess.run(
        ["script", "-q", str(out), str(MAHOJIN), *shlex.split(spell)],
        env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, check=True,
    )
    data = out.read_bytes()
    rows = int(re.search(rb"\x1b_G[^;]*,r=(\d+),", data).group(1))
    # 最後の画像の後、カーソルを画像の下へ動かした先に出た文字
    after = re.split(rb"\x1b\[\d+B\r", data)[-1].decode().strip()
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
        sys.exit(f"mahojin から画像を受け取れませんでした: {spell}")
    return paths, rows, after


def slug(s: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", s.lower()).strip("-")


def terminal_frame(dest: Path, typed: str, circle: Path | None, rows: int, output: list[str]):
    """ターミナル 1 コマを描く。魔法陣は端末と同じく rows 行ぶんの高さで貼る。"""
    cmd = [
        "magick", "-size", f"{W}x{H}", f"xc:{BG}",
        "-fill", BAR, "-draw", f"rectangle 0,0 {W},32",
        "-fill", "#f38ba8", "-draw", "circle 20,16 26,16",
        "-fill", "#f9e2af", "-draw", "circle 42,16 48,16",
        "-fill", "#a6e3a1", "-draw", "circle 64,16 70,16",
        "-font", FONT, "-pointsize", "14", "-fill", "#6c7086",
        "-annotate", "+330+21", "mahojin",
        "-pointsize", "17",
        "-fill", "#a6e3a1", "-annotate", "+20+60", "$",
        "-fill", "#cdd6f4", "-annotate", "+40+60", typed,
    ]
    size = rows * ROW_PX
    if circle is not None:
        cmd += ["(", str(circle), "-resize", f"{size}x{size}", ")",
                "-geometry", f"+{(W - size) // 2}+{CIRCLE_Y}", "-composite"]
    y = CIRCLE_Y + size + 28
    for line in output:
        if line:
            cmd += ["-pointsize", "15", "-fill", "#cdd6f4", "-annotate", f"+20+{y}", line]
        y += LINE_H
    cmd.append(str(dest))
    subprocess.run(cmd, check=True)


def build_demo(stub_dir: Path, work: Path):
    frames: list[tuple[Path, int]] = []  # (画像, 1/100 秒単位の表示時間)
    n = 0

    def add(typed, circle, output, delay, rows=0):
        nonlocal n
        p = work / f"demo-{n:04d}.png"
        terminal_frame(p, typed, circle, rows, output)
        frames.append((p, delay))
        n += 1

    for spell, output in DEMOS:
        circles, rows, tier = capture(spell, stub_dir, work)
        line = f"mahojin {spell}"
        for k in range(0, len(line) + 1, 2):
            add(line[:k], None, [], 5)
        add(line, None, [], 35)
        # mahojin と同じく 16 コマをおよそ 45ms 間隔で
        for c in circles:
            add(line, c, [], 5, rows)
        head = [tier] if tier else []
        add(line, circles[-1], head, 20, rows)
        add(line, circles[-1], head + output, 230, rows)

    args = ["magick", "-loop", "0"]
    for p, delay in frames:
        args += ["-delay", str(delay), str(p)]
    args += ["-layers", "Optimize", str(ASSETS / "demo.gif")]
    subprocess.run(args, check=True)


def build_gallery(stub_dir: Path, work: Path):
    """--share が書き出す大きな画像を並べる。--share はコマンドを実行しない。"""
    tiles = []
    for spell in GALLERY:
        res = subprocess.run(
            [str(MAHOJIN), "--share", *shlex.split(spell)],
            cwd=work, env={"PATH": "/usr/bin:/bin", "MAHOJIN_LOCALE": "en", "MAHOJIN_GRIMOIRE": "off",
                 "MAHOJIN_NOW": ORDINARY_DAY, "HOME": "/tmp"},
            stdin=subprocess.DEVNULL, capture_output=True, text=True, check=True,
        )
        image = re.search(r"image\s+(\S+)", res.stderr).group(1)
        tier = re.search(r" an? (.+?) circle unfolded", res.stdout).group(1)
        tiles += ["-label", f"{spell}\n{tier}", str(work / image)]
    subprocess.run([
        "magick", "montage", *tiles,
        "-tile", "4x2", "-geometry", "320x320+8+8",
        "-font", FONT, "-pointsize", "17",
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
