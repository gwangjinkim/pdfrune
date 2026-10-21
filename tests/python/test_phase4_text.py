import pdfrune
from pdfrune import Rect


TEXT_PDF_BYTES = (
    b"%PDF-1.7\n"
    b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n"
    b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>\nendobj\n"
    b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> /Contents 4 0 R >>\nendobj\n"
    b"4 0 obj\n<< /Length 260 >>\nstream\n"
    b"BT\n"
    b"/F1 24 Tf\n"
    b"72 720 Td\n"
    b"(Welcome to pdfrune) Tj\n"
    b"/F1 12 Tf\n"
    b"0 -36 Td\n"
    b"[(Fast ) -50 (Rust ) -300 (PDF ) 10 (Engine)] TJ\n"
    b"14 TL\n"
    b"T*\n"
    b"(Second line in paragraph one.) Tj\n"
    b"0 -60 Td\n"
    b"(Paragraph two has searchable needles.) Tj\n"
    b"/F2 12 Tf\n"
    b"0 -40 Td\n"
    b"<000100020003> Tj\n"
    b"ET\n"
    b"endstream\nendobj\n"
    b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n"
    b"6 0 obj\n<< /Type /Font /Subtype /Type0 /BaseFont /CustomFont /ToUnicode 7 0 R >>\nendobj\n"
    b"7 0 obj\n<< /Length 160 >>\nstream\n"
    b"/CIDInit /ProcSet findresource begin\n"
    b"12 dict begin\n"
    b"begincmap\n"
    b"1 begincodespacerange\n"
    b"<0000> <FFFF>\n"
    b"endcodespacerange\n"
    b"3 beginbfchar\n"
    b"<0001> <0043>\n"
    b"<0002> <004D>\n"
    b"<0003> <0050>\n"
    b"endbfchar\n"
    b"endcmap\n"
    b"endstream\nendobj\n"
    b"trailer\n<< /Root 1 0 R /Size 8 >>\n"
    b"startxref\n0\n%%EOF"
)


def test_extract_plain_text_and_tounicode_cmap():
    with pdfrune.open(stream=TEXT_PDF_BYTES) as doc:
        page = doc[0]
        text = page.extract_text()
        assert "Welcome to pdfrune" in text
        assert "Fast Rust PDF Engine" in text
        assert "Second line in paragraph one." in text
        assert "Paragraph two has searchable needles." in text
        # Verify ToUnicode CMap decoded <000100020003> -> "CMP"
        assert "CMP" in text

        # Verify get_text("text") compatibility
        assert page.get_text("text") == text


def test_extract_words_and_coordinates():
    with pdfrune.open(stream=TEXT_PDF_BYTES) as doc:
        page = doc[0]
        words = page.extract_words()
        assert len(words) >= 14

        word_strings = [w[4] for w in words]
        assert "Welcome" in word_strings
        assert "pdfrune" in word_strings
        assert "Engine" in word_strings
        assert "CMP" in word_strings

        # Every word tuple is (x0, y0, x1, y1, word, block_no, line_no, word_no)
        for x0, y0, x1, y1, w_text, block_no, line_no, word_no in words:
            assert x1 > x0
            assert y1 > y0
            assert 0.0 <= x0 < 612.0
            assert 0.0 <= y0 < 792.0
            assert isinstance(w_text, str) and len(w_text) > 0
            assert block_no >= 0
            assert line_no >= 0
            assert word_no >= 0

        # "Welcome" at y=720 in PDF space (bottom-up) is near y=72 in top-down space,
        # which should have a smaller y0 than "Paragraph" (at y=624 in PDF space -> ~168 top-down)
        welcome_w = next(w for w in words if w[4] == "Welcome")
        para_w = next(w for w in words if w[4] == "Paragraph")
        assert welcome_w[1] < para_w[1]


def test_extract_blocks():
    with pdfrune.open(stream=TEXT_PDF_BYTES) as doc:
        page = doc[0]
        blocks = page.extract_blocks()
        assert len(blocks) >= 3

        # Each block is (x0, y0, x1, y1, text, block_no, block_type)
        assert "Welcome to pdfrune" in blocks[0][4]
        assert "Fast Rust PDF Engine" in blocks[1][4]
        assert "Second line in paragraph one." in blocks[1][4]
        assert any("Paragraph two has searchable needles." in b[4] for b in blocks)


def test_page_search():
    with pdfrune.open(stream=TEXT_PDF_BYTES) as doc:
        page = doc[0]
        hits = page.search("pdfrune")
        assert len(hits) == 1
        assert isinstance(hits[0], Rect)
        assert hits[0].width > 10.0
        assert hits[0].height > 10.0

        # Case-insensitive and multi-word search
        multi_hits = page.search("searchable needles")
        assert len(multi_hits) >= 1
        assert multi_hits[0].width > 20.0

        # PyMuPDF alias search_for
        assert page.search_for("pdfrune") == hits
