import pathlib
import pdfrune
from pdfrune import Pixmap, Point, Rect


def test_render_pixmap_dimensions_and_pixels(tmp_path: pathlib.Path):
    doc = pdfrune.open()
    page = doc.new_page(width=200.0, height=100.0)
    # Draw a solid red rectangle from (20, 20) to (80, 60)
    page.draw_rect(
        Rect(20.0, 20.0, 80.0, 60.0),
        color=(1.0, 0.0, 0.0),
        fill=(1.0, 0.0, 0.0),
        width=1.0,
    )
    page.insert_text(Point(20.0, 80.0), "Render Test", fontsize=14.0, color=(0.0, 0.0, 1.0))

    pix = page.render(dpi=72.0)
    assert isinstance(pix, Pixmap)
    assert pix.width == 200
    assert pix.height == 100
    assert pix.n == 4
    assert pix.stride == 800
    assert len(pix.samples) == 200 * 100 * 4

    # Background pixel at (5, 5) should be white
    assert pix.pixel(5, 5) == (255, 255, 255, 255)

    # Pixel inside the red filled rectangle at (50, 40) should be pure red
    r, g, b, a = pix.pixel(50, 40)
    assert r > 240 and g < 15 and b < 15 and a == 255

    # Higher DPI rendering (144 DPI = 2x scale)
    pix_hi = page.get_pixmap(dpi=144.0)
    assert pix_hi.width == 400
    assert pix_hi.height == 200

    # PNG export in-memory and to disk
    png_bytes = pix.to_png()
    assert png_bytes.startswith(b"\x89PNG\r\n\x1a\n")
    assert pix.tobytes("png") == png_bytes

    png_path = tmp_path / "rendered.png"
    pix.save_png(str(png_path))
    assert png_path.exists()
    assert png_path.read_bytes().startswith(b"\x89PNG\r\n\x1a\n")


def test_svg_export():
    doc = pdfrune.open()
    page = doc.new_page(width=400.0, height=300.0)
    page.draw_rect(Rect(10.0, 10.0, 100.0, 50.0), color=(0.0, 0.5, 1.0), fill=(0.9, 0.9, 0.9))
    page.insert_text(Point(20.0, 100.0), "Vector SVG Output", fontsize=16.0)

    svg = page.to_svg()
    assert "<svg" in svg
    assert 'width="400"' in svg
    assert 'height="300"' in svg
    assert "Vector SVG Output" in svg
    assert page.get_svg_image() == svg
