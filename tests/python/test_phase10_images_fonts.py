import pathlib
import pdfrune
from pdfrune import Pixmap, Rect


def create_solid_red_png() -> bytes:
    # Render a small 20x16 page with a solid red rectangle and export to PNG bytes
    doc = pdfrune.open()
    p = doc.new_page(width=20.0, height=16.0)
    p.draw_rect(Rect(0.0, 0.0, 20.0, 16.0), color=None, fill=(1.0, 0.0, 0.0))
    pix = p.render(dpi=72.0)
    return pix.to_png()


def test_insert_get_and_extract_images(tmp_path: pathlib.Path):
    png_bytes = create_solid_red_png()
    img_path = tmp_path / "red.png"
    img_path.write_bytes(png_bytes)

    doc = pdfrune.open()
    page = doc.new_page(width=200.0, height=200.0)
    assert page.get_images() == []

    # Insert image from PNG stream bytes
    xref1 = page.insert_image(Rect(20.0, 30.0, 120.0, 110.0), stream=png_bytes)
    assert xref1 > 0

    # Insert second image from file path
    xref2 = page.insert_image(Rect(20.0, 120.0, 80.0, 170.0), filename=str(img_path))
    assert xref2 > xref1

    images = page.get_images()
    assert len(images) == 2
    # Tuple format: (xref, smask, width, height, bpc, colorspace, alt, name, filter)
    first = images[0]
    assert first[0] == xref1
    assert first[2] == 20
    assert first[3] == 16
    assert first[4] == 8
    assert first[5] == "DeviceRGB"

    # Extract image via doc.extract_image(xref)
    extracted = doc.extract_image(xref1)
    assert extracted["ext"] == "png"
    assert extracted["width"] == 20
    assert extracted["height"] == 16
    assert extracted["bpc"] == 8
    assert extracted["image"].startswith(b"\x89PNG\r\n\x1a\n")

    # Roundtrip serialize & reopen
    raw = doc.to_bytes()
    with pdfrune.open(raw) as reopened:
        r_imgs = reopened[0].get_images()
        assert len(r_imgs) == 2
        r_ext = reopened.extract_image(r_imgs[0][0])
        assert r_ext["width"] == 20
        assert r_ext["height"] == 16


def test_render_embedded_image_xobject():
    png_bytes = create_solid_red_png()
    doc = pdfrune.open()
    page = doc.new_page(width=100.0, height=100.0)
    # Place the solid red image at Rect(10, 10, 60, 60)
    page.insert_image(Rect(10.0, 10.0, 60.0, 60.0), stream=png_bytes)

    pix = page.render(dpi=72.0)
    assert isinstance(pix, Pixmap)
    # Center of image at (35, 35) should be red (R > 200, G < 30, B < 30)
    r, g, b, a = pix.pixel(35, 35)
    assert r > 200 and g < 30 and b < 30 and a == 255
    # Outside image at (85, 85) should be white background
    assert pix.pixel(85, 85) == (255, 255, 255, 255)
