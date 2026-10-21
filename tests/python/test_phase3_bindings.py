import pathlib
import pytest
import pdfrune
from pdfrune import Document, Matrix, Page, Point, Rect


SAMPLE_PDF_BYTES = (
    b"%PDF-1.7\n"
    b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n"
    b"2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 /MediaBox [0 0 612 792] >>\nendobj\n"
    b"3 0 obj\n<< /Type /Page /Parent 2 0 R /CropBox [10 20 600 780] >>\nendobj\n"
    b"4 0 obj\n<< /Type /Page /Parent 2 0 R /Rotate 90 >>\nendobj\n"
    b"5 0 obj\n<< /Title (Phase 3 Spec) /Author (Ada Lovelace) /Subject (Bindings) >>\nendobj\n"
    b"trailer\n<< /Root 1 0 R /Info 5 0 R /Size 6 >>\n"
    b"startxref\n0\n%%EOF"
)


def test_point_geometry():
    p1 = Point(3.0, 4.0)
    p2 = Point(0.0, 0.0)
    assert p1.x == 3.0
    assert p1.y == 4.0
    assert tuple(p1) == (3.0, 4.0)
    assert p1.distance_to(p2) == pytest.approx(5.0)
    assert p1 == Point(3.0, 4.0)
    assert "Point(3" in repr(p1)


def test_rect_geometry():
    r1 = Rect(0.0, 0.0, 100.0, 200.0)
    assert r1.width == 100.0
    assert r1.height == 200.0
    assert r1.area == 20000.0
    assert not r1.is_empty
    assert tuple(r1) == (0.0, 0.0, 100.0, 200.0)
    assert r1[2] == 100.0
    assert r1.contains(Point(50.0, 50.0))
    assert not r1.contains(Point(150.0, 50.0))

    r2 = Rect(50.0, 100.0, 150.0, 300.0)
    assert r1.intersects(r2)
    inter = r1.intersect(r2)
    assert tuple(inter) == (50.0, 100.0, 100.0, 200.0)
    uni = r1.union(r2)
    assert tuple(uni) == (0.0, 0.0, 150.0, 300.0)


def test_matrix_geometry():
    m = Matrix(2.0, 0.0, 0.0, 3.0, 10.0, 20.0)
    pt = m.transform_point(Point(5.0, 4.0))
    assert pt.x == pytest.approx(20.0)
    assert pt.y == pytest.approx(32.0)


def test_document_open_bytes_and_context_manager(tmp_path: pathlib.Path):
    pdf_file = tmp_path / "sample.pdf"
    pdf_file.write_bytes(SAMPLE_PDF_BYTES)

    # Open from path
    with pdfrune.open(str(pdf_file)) as doc:
        assert isinstance(doc, Document)
        assert not doc.is_closed
        assert doc.page_count == 2
        assert len(doc) == 2

        meta = doc.metadata
        assert meta["title"] == "Phase 3 Spec"
        assert meta["author"] == "Ada Lovelace"
        assert meta["subject"] == "Bindings"
        assert meta["format"] == "PDF 1.7"

        # Page indexing & negative indexing
        p0 = doc[0]
        assert isinstance(p0, Page)
        assert p0.number == 0
        assert tuple(p0.mediabox) == (0.0, 0.0, 612.0, 792.0)
        assert tuple(p0.cropbox) == (10.0, 20.0, 600.0, 780.0)
        assert p0.width == pytest.approx(590.0)
        assert p0.height == pytest.approx(760.0)
        assert p0.rotation == 0

        p1 = doc[-1]
        assert p1.number == 1
        assert p1.rotation == 90
        assert p1.width == pytest.approx(792.0)
        assert p1.height == pytest.approx(612.0)

        # Iteration and slicing
        pages = list(doc)
        assert len(pages) == 2
        sliced = doc[0:1]
        assert len(sliced) == 1
        assert sliced[0].number == 0

    assert doc.is_closed


def test_document_open_from_memory_and_mutate_metadata():
    doc = pdfrune.open(stream=SAMPLE_PDF_BYTES)
    assert len(doc) == 2
    doc.set_metadata({"title": "Updated Title", "author": "Rust Engine"})
    assert doc.metadata["title"] == "Updated Title"
    assert doc.metadata["author"] == "Rust Engine"
    assert not doc.is_encrypted
    assert not doc.needs_pass
    assert doc.authenticate("") is True
