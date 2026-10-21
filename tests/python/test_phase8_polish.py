import pytest
import pdfrune
from pdfrune import Point, Rect


def test_builtin_rust_benchmark():
    metrics = pdfrune.benchmark(pages=20)
    assert metrics["pages"] == 20
    assert metrics["parse_ms"] >= 0.0
    assert metrics["extract_ms"] >= 0.0
    assert metrics["search_ms"] >= 0.0
    assert metrics["markdown_ms"] >= 0.0
    assert metrics["render_ms"] >= 0.0
    assert metrics["pages_per_sec_extract"] > 0.0
    assert metrics["pages_per_sec_render"] > 0.0


def test_continuous_loop_memory_stability():
    # Stress test 100 iterations of create -> insert -> serialize -> parse -> extract -> render -> close
    for i in range(100):
        with pdfrune.open() as doc:
            page = doc.new_page(width=400.0, height=300.0)
            page.draw_rect(Rect(10.0, 10.0, 200.0, 100.0), fill=(0.2, 0.4, 0.8))
            page.insert_text(Point(20.0, 50.0), f"Iteration {i} stability check", fontsize=14.0)
            raw = doc.to_bytes(deflate=True)

        with pdfrune.open(raw) as reopened:
            assert reopened.page_count == 1
            text = reopened[0].extract_text()
            assert f"Iteration {i}" in text
            pix = reopened[0].render(dpi=36.0)
            assert pix.width == 200
            assert pix.height == 150


def test_corrupt_and_malformed_pdf_resilience():
    # Completely empty bytes or non-PDF bytes should raise ValueError cleanly without panicking
    with pytest.raises(ValueError):
        pdfrune.open(b"")

    with pytest.raises(ValueError):
        pdfrune.open(b"NOT_A_PDF_HEADER_AT_ALL")

    # Truncated PDF with broken XREF should be auto-repaired by SIMD scanner if objects exist
    broken_xref_pdf = (
        b"%PDF-1.7\n"
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n"
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n"
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Contents 4 0 R >>\nendobj\n"
        b"4 0 obj\n<< /Length 44 >>\nstream\nBT /F1 12 Tf 20 100 Td (Auto-repaired) Tj ET\nendstream\nendobj\n"
        b"xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Root 1 0 R >>\nstartxref\n999999\n%%EOF"
    )
    with pdfrune.open(broken_xref_pdf) as doc:
        assert doc.page_count == 1
        assert "Auto-repaired" in doc[0].extract_text()
