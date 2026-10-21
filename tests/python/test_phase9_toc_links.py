import pdfrune
from pdfrune import Point, Rect


def test_toc_roundtrip():
    doc = pdfrune.open()
    p1 = doc.new_page()
    p1.insert_text(Point(72.0, 72.0), "Chapter 1: Introduction", fontsize=20.0)
    p2 = doc.new_page()
    p2.insert_text(Point(72.0, 72.0), "Chapter 2: Methods", fontsize=20.0)
    p3 = doc.new_page()
    p3.insert_text(Point(72.0, 72.0), "Chapter 3: Results", fontsize=20.0)

    assert doc.get_toc() == []

    toc = [
        [1, "Chapter 1: Introduction", 1],
        [2, "Section 1.1: Background", 1],
        [1, "Chapter 2: Methods", 2],
        [2, "Section 2.1: Setup", 2],
        [3, "Subsection 2.1.1: Hardware", 2],
        [1, "Chapter 3: Results", 3],
    ]
    count = doc.set_toc(toc)
    assert count == 6
    assert doc.get_toc() == toc

    # Serialize and reopen to verify ISO 32000 /Outlines tree persistence
    raw = doc.to_bytes()
    with pdfrune.open(raw) as reopened:
        assert reopened.get_toc() == toc


def test_page_hyperlinks_uri_and_goto_roundtrip():
    doc = pdfrune.open()
    p0 = doc.new_page(width=612.0, height=792.0)
    p1 = doc.new_page(width=612.0, height=792.0)
    p0.insert_text(Point(72.0, 100.0), "Click here to visit Rust", fontsize=14.0)
    p1.insert_text(Point(72.0, 100.0), "Target Page 2", fontsize=14.0)

    assert p0.get_links() == []

    # Insert external URI link
    p0.insert_link(
        {
            "from": Rect(72.0, 85.0, 250.0, 110.0),
            "uri": "https://www.rust-lang.org",
        }
    )
    # Insert internal GoTo link to page index 1
    p0.insert_link(
        {
            "from": Rect(72.0, 140.0, 220.0, 165.0),
            "page": 1,
        }
    )

    links = p0.get_links()
    assert len(links) == 2
    assert links[0]["kind"] == "uri"
    assert links[0]["uri"] == "https://www.rust-lang.org"
    assert abs(links[0]["from"].x0 - 72.0) < 1e-3
    assert abs(links[0]["from"].y0 - 85.0) < 1e-3

    assert links[1]["kind"] == "goto"
    assert links[1]["page"] == 1

    # Roundtrip serialize & reopen
    raw = doc.to_bytes()
    with pdfrune.open(raw) as reopened:
        r_links = reopened[0].get_links()
        assert len(r_links) == 2
        assert r_links[0]["uri"] == "https://www.rust-lang.org"
        assert r_links[1]["page"] == 1

        # Delete first link
        reopened[0].delete_link(r_links[0])
        remaining = reopened[0].get_links()
        assert len(remaining) == 1
        assert remaining[0]["kind"] == "goto"
