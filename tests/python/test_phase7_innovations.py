import pathlib
import pdfrune
from pdfrune import Point, Rect


def build_structured_doc() -> pdfrune.Document:
    doc = pdfrune.open()
    p1 = doc.new_page(width=612.0, height=792.0)
    p1.insert_text(Point(72.0, 72.0), "Architecture Overview", fontsize=24.0)
    p1.insert_text(
        Point(72.0, 120.0),
        "pdfrune is a memory-safe PDF engine written in Rust.",
        fontsize=12.0,
    )
    p1.insert_text(Point(72.0, 160.0), "Key Capabilities", fontsize=18.0)
    p1.insert_text(Point(72.0, 195.0), "- Zero AGPL contamination", fontsize=12.0)
    p1.insert_text(Point(72.0, 215.0), "- Parallel rayon execution", fontsize=12.0)

    p2 = doc.new_page(width=612.0, height=792.0)
    p2.insert_text(Point(72.0, 72.0), "Performance Benchmarks", fontsize=24.0)
    p2.insert_text(
        Point(72.0, 120.0),
        "SIMD acceleration enables sub-millisecond search across 1000 pages.",
        fontsize=12.0,
    )
    return doc


def test_markdown_and_html_export():
    doc = build_structured_doc()
    md_p0 = doc[0].to_markdown()
    assert "# Architecture Overview" in md_p0
    assert "## Key Capabilities" in md_p0
    assert "- Zero AGPL contamination" in md_p0
    assert "memory-safe PDF engine" in md_p0

    full_md = doc.to_markdown()
    assert "# Architecture Overview" in full_md
    assert "# Performance Benchmarks" in full_md

    html_p0 = doc[0].to_html()
    assert "<h1>Architecture Overview</h1>" in html_p0
    assert "<h2>Key Capabilities</h2>" in html_p0
    assert "<li>Zero AGPL contamination</li>" in html_p0


def test_rag_semantic_chunks():
    doc = build_structured_doc()
    chunks = doc.to_rag_chunks(max_chars=120)
    assert len(chunks) >= 2

    for chunk in chunks:
        assert "chunk_id" in chunk
        assert "page_number" in chunk
        assert "bbox" in chunk
        assert "text" in chunk
        assert "section_heading" in chunk
        x0, y0, x1, y1 = chunk["bbox"]
        assert x1 > x0 and y1 > y0
        assert len(chunk["text"]) > 0

    assert any(c["page_number"] == 0 for c in chunks)
    assert any(c["page_number"] == 1 for c in chunks)
    assert any("Architecture Overview" in c["section_heading"] for c in chunks)


def test_simd_and_regex_document_search():
    doc = build_structured_doc()
    # Exact / case-insensitive SIMD search across all pages
    hits = doc.search_all("SIMD acceleration")
    assert len(hits) == 1
    page_no, rects = hits[0]
    assert page_no == 1
    assert len(rects) == 1
    assert isinstance(rects[0], Rect)

    # Regex search across all pages (e.g. matching 4-digit numbers like 1000)
    regex_hits = doc.search_all(r"\b\d{4}\b", regex=True)
    assert len(regex_hits) == 1
    assert regex_hits[0][0] == 1


def test_batch_extract_and_batch_render(tmp_path: pathlib.Path):
    d1 = build_structured_doc()
    d2 = pdfrune.open()
    p = d2.new_page()
    p.insert_text(Point(72.0, 100.0), "Second batch document content", fontsize=14.0)

    p1_path = tmp_path / "b1.pdf"
    d1.save(str(p1_path))

    texts = pdfrune.batch_extract([str(p1_path), d2.to_bytes()])
    assert len(texts) == 2
    assert "Architecture Overview" in texts[0]
    assert "Second batch document content" in texts[1]

    pixmaps = pdfrune.batch_render(d1.to_bytes(), dpi=72.0)
    assert len(pixmaps) == 2
    assert pixmaps[0].width == 612
    assert pixmaps[1].height == 792


def test_fitz_compatibility_shim():
    import pdfrune.fitz as fitz
    from pdfrune.compat import fitz as fitz2

    assert fitz is fitz2
    doc = build_structured_doc()
    raw = doc.to_bytes()

    with fitz.open(stream=raw, filetype="pdf") as fdoc:
        assert fdoc.page_count == 2
        page = fdoc.load_page(0)
        assert "Architecture Overview" in page.get_text("text")
        pix = page.get_pixmap(dpi=72)
        assert pix.width == 612
        rects = page.search_for("Rust")
        assert len(rects) == 1
