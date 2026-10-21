import pathlib
import pdfrune
from pdfrune import Point, Rect


def make_test_doc(title: str, page_texts: list[str]) -> pdfrune.Document:
    doc = pdfrune.open()
    doc.set_metadata({"title": title, "author": "pdfrune test"})
    for text in page_texts:
        page = doc.new_page(width=612.0, height=792.0)
        page.insert_text(Point(72.0, 100.0), text, fontsize=14.0)
    return doc


def test_create_new_pdf_insert_text_and_roundtrip(tmp_path: pathlib.Path):
    doc = pdfrune.open()
    assert len(doc) == 0

    page = doc.new_page(width=595.0, height=842.0)
    assert len(doc) == 1
    assert page.width == 595.0
    assert page.height == 842.0

    page.draw_rect(Rect(50.0, 50.0, 400.0, 150.0))
    page.insert_text(Point(72.0, 96.0), "Hello from newly created PDF!", fontsize=16.0)
    doc.set_metadata({"title": "Created From Scratch", "author": "pdfrune"})

    raw_bytes = doc.to_bytes()
    assert raw_bytes.startswith(b"%PDF-1.7")
    assert b"%%EOF" in raw_bytes

    out_path = tmp_path / "created.pdf"
    doc.save(str(out_path), deflate=True)
    assert out_path.exists()

    with pdfrune.open(str(out_path)) as reloaded:
        assert len(reloaded) == 1
        assert reloaded.metadata["title"] == "Created From Scratch"
        assert reloaded.metadata["author"] == "pdfrune"
        p0 = reloaded[0]
        assert p0.width == 595.0
        assert p0.height == 842.0
        assert "Hello from newly created PDF!" in p0.extract_text()


def test_merge_and_insert_pdf():
    doc1 = make_test_doc("Doc 1", ["Alpha Page 1", "Alpha Page 2"])
    doc2 = make_test_doc("Doc 2", ["Beta Page 1", "Beta Page 2"])

    doc1.insert_pdf(doc2)
    assert len(doc1) == 4

    reloaded = pdfrune.open(stream=doc1.to_bytes())
    assert len(reloaded) == 4
    assert "Alpha Page 1" in reloaded[0].extract_text()
    assert "Alpha Page 2" in reloaded[1].extract_text()
    assert "Beta Page 1" in reloaded[2].extract_text()
    assert "Beta Page 2" in reloaded[3].extract_text()

    # Test top-level pdfrune.merge([d1, d2])
    d_a = make_test_doc("A", ["First Merge"])
    d_b = make_test_doc("B", ["Second Merge"])
    merged = pdfrune.merge([d_a, d_b])
    assert len(merged) == 2
    assert "First Merge" in merged[0].extract_text()
    assert "Second Merge" in merged[1].extract_text()


def test_delete_select_and_extract_pages():
    doc = make_test_doc("Multi", ["Page Zero", "Page One", "Page Two"])
    assert len(doc) == 3

    # Extract subset into new Document without mutating original
    sub = doc.extract_pages([2, 0])
    assert len(sub) == 2
    assert "Page Two" in sub[0].extract_text()
    assert "Page Zero" in sub[1].extract_text()
    assert len(doc) == 3

    # Delete middle page
    doc.delete_page(1)
    assert len(doc) == 2
    assert "Page Zero" in doc[0].extract_text()
    assert "Page Two" in doc[1].extract_text()

    # Reorder pages with select()
    doc.select([1, 0])
    reloaded = pdfrune.open(stream=doc.to_bytes())
    assert len(reloaded) == 2
    assert "Page Two" in reloaded[0].extract_text()
    assert "Page Zero" in reloaded[1].extract_text()


def test_rotate_and_crop_persistence():
    doc = make_test_doc("Rotate", ["Rotated Content"])
    page = doc[0]
    page.rotation = 270
    page.cropbox = Rect(10.0, 10.0, 500.0, 700.0)

    reloaded = pdfrune.open(stream=doc.to_bytes())
    p0 = reloaded[0]
    assert p0.rotation == 270
    assert tuple(p0.cropbox) == (10.0, 10.0, 500.0, 700.0)
