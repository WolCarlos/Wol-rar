from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets"
ASSETS.mkdir(exist_ok=True)
size = 256
image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
draw = ImageDraw.Draw(image)
draw.rounded_rectangle((8, 8, 248, 248), radius=54, fill="#173f35")
draw.rounded_rectangle((39, 69, 217, 207), radius=20, fill="#f2f6f4")
draw.polygon(((39, 94), (91, 94), (110, 69), (167, 69), (185, 94), (217, 94), (217, 119), (39, 119)), fill="#8dd8bd")
draw.rounded_rectangle((70, 130, 186, 146), radius=7, fill="#b7c9c2")
draw.rounded_rectangle((70, 157, 161, 173), radius=7, fill="#d0ddd8")
try:
    font = ImageFont.truetype("DejaVuSans-Bold.ttf", 53)
except OSError:
    font = ImageFont.load_default()
draw.text((128, 36), "W", font=font, fill="#ffffff", anchor="mm")
image.save(ASSETS / "wol-rar.png")
image.save(ASSETS / "wol-rar.ico", sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
print(ASSETS / "wol-rar.png")
