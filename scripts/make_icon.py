from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw


ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets"
ASSETS.mkdir(exist_ok=True)
SCALE = 2


def rounded(draw: ImageDraw.ImageDraw, box, radius, fill, outline=None, width=1):
    draw.rounded_rectangle(tuple(v * SCALE for v in box), radius=radius * SCALE, fill=fill, outline=outline, width=width * SCALE)


def polygon(draw: ImageDraw.ImageDraw, points, fill):
    draw.polygon([(x * SCALE, y * SCALE) for x, y in points], fill=fill)


def line(draw: ImageDraw.ImageDraw, points, fill, width=4):
    draw.line([(x * SCALE, y * SCALE) for x, y in points], fill=fill, width=width * SCALE, joint="curve")


def draw_stack(draw: ImageDraw.ImageDraw, x=0, y=0, scale=1.0):
    def point(px, py):
        return x + px * scale, y + py * scale

    colors = [
        ("#37a88b", "#7fd4b8", "#21745f"),
        ("#4387c7", "#83b9e8", "#2c5f94"),
        ("#e4ad45", "#f6d785", "#a97324"),
    ]
    for index, (front, top, side) in enumerate(colors):
        yy = 78 - index * 31
        polygon(draw, [point(8, yy), point(27, yy - 12), point(67, yy - 12), point(48, yy)], top)
        rounded(draw, (point(8, yy)[0], point(8, yy)[1], point(48, yy + 25)[0], point(48, yy + 25)[1]), 3, front)
        polygon(draw, [point(48, yy), point(67, yy - 12), point(67, yy + 13), point(48, yy + 25)], side)
        line(draw, [point(15, yy + 8), point(39, yy + 8)], "#e9f4ef", 2)
    polygon(draw, [point(29, 13), point(39, 13), point(39, 108), point(29, 108)], "#f3eee0")
    polygon(draw, [point(39, 13), point(45, 9), point(45, 104), point(39, 108)], "#c9bd9c")


def save_icon(image: Image.Image, name: str) -> None:
    image.save(ASSETS / f"{name}.png")
    image.save(
        ASSETS / f"{name}.ico",
        sizes=[(16, 16), (20, 20), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )


def make_app_icon() -> None:
    image = Image.new("RGBA", (256 * SCALE, 256 * SCALE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    rounded(draw, (8, 8, 248, 248), 54, "#173f35")
    draw.ellipse((44 * SCALE, 211 * SCALE, 218 * SCALE, 230 * SCALE), fill="#0d2a23")
    draw_stack(draw, 50, 35, 1.85)
    save_icon(image.resize((256, 256), Image.Resampling.LANCZOS), "wol-rar")


def arrow(draw: ImageDraw.ImageDraw, points, color="#ffffff", width=5):
    line(draw, points, color, width)
    (x1, y1), (x2, y2), (x3, y3) = points[-3:]
    polygon(draw, [(x1, y1), (x2, y2), (x3, y3)], color)


def make_menu_icons() -> None:
    for name in ("open", "create", "extract", "test", "update", "info", "installer"):
        image = Image.new("RGBA", (64 * SCALE, 64 * SCALE), (0, 0, 0, 0))
        draw = ImageDraw.Draw(image)
        rounded(draw, (2, 2, 62, 62), 14, "#173f35")
        draw_stack(draw, 4, 11, 0.43)
        badge = "#f3f7f5"
        if name == "open":
            polygon(draw, [(34, 23), (43, 23), (47, 28), (59, 28), (57, 49), (33, 49)], "#f2c85f")
            polygon(draw, [(33, 31), (58, 31), (55, 49), (33, 49)], "#f7d983")
            arrow(draw, [(45, 45), (45, 33), (39, 39), (45, 33), (51, 39)], "#ffffff", 3)
        elif name in {"create", "installer"}:
            arrow(draw, [(47, 19), (47, 44), (39, 36), (47, 44), (55, 36)], badge, 4)
            line(draw, [(35, 50), (58, 50)], badge, 4)
            if name == "installer":
                line(draw, [(54, 13), (54, 25)], "#f2c85f", 4)
                line(draw, [(48, 19), (60, 19)], "#f2c85f", 4)
        elif name == "extract":
            line(draw, [(35, 49), (58, 49)], badge, 4)
            arrow(draw, [(47, 47), (47, 20), (39, 28), (47, 20), (55, 28)], badge, 4)
        elif name == "test":
            polygon(draw, [(47, 17), (59, 22), (57, 37), (52, 45), (47, 49), (41, 45), (36, 37), (35, 22)], "#37a88b")
            line(draw, [(40, 32), (45, 37), (54, 27)], "#ffffff", 4)
        elif name == "update":
            line(draw, [(37, 25), (43, 20), (51, 20), (57, 26)], badge, 4)
            arrow(draw, [(57, 26), (57, 18), (57, 26), (50, 26)], badge, 4)
            line(draw, [(56, 39), (50, 45), (42, 45), (36, 39)], badge, 4)
            arrow(draw, [(36, 39), (36, 47), (36, 39), (43, 39)], badge, 4)
        elif name == "info":
            draw.ellipse((37 * SCALE, 17 * SCALE, 57 * SCALE, 47 * SCALE), fill="#4387c7")
            rounded(draw, (45, 21, 49, 25), 2, "#ffffff")
            rounded(draw, (45, 29, 49, 41), 2, "#ffffff")
        save_icon(image.resize((64, 64), Image.Resampling.LANCZOS), f"menu-{name}")


if __name__ == "__main__":
    make_app_icon()
    make_menu_icons()
    print(f"Icone create in {ASSETS}")
