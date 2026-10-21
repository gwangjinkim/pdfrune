import pdfrune
from pdfrune import Point, Rect


def test_find_tables_extraction_and_markdown():
    doc = pdfrune.open()
    page = doc.new_page(width=612.0, height=792.0)

    # Normal paragraph before the table
    page.insert_text(Point(72.0, 60.0), "Quarterly Financial Summary", fontsize=18.0)

    # 3-row x 3-column aligned table
    rows = [
        ("Quarter", "Revenue", "Growth"),
        ("Q1-2026", "$12.4M", "+18%"),
        ("Q2-2026", "$15.1M", "+22%"),
        ("Q3-2026", "$19.8M", "+31%"),
    ]
    y = 120.0
    for c0, c1, c2 in rows:
        page.insert_text(Point(72.0, y), c0, fontsize=12.0)
        page.insert_text(Point(220.0, y), c1, fontsize=12.0)
        page.insert_text(Point(360.0, y), c2, fontsize=12.0)
        y += 28.0

    tabs = page.find_tables()
    assert len(tabs) == 1
    table = tabs[0]
    assert table.row_count == 4
    assert table.col_count == 3
    assert isinstance(table.bbox, Rect)
    assert table.header == ["Quarter", "Revenue", "Growth"]

    grid = table.extract()
    assert grid == [
        ["Quarter", "Revenue", "Growth"],
        ["Q1-2026", "$12.4M", "+18%"],
        ["Q2-2026", "$15.1M", "+22%"],
        ["Q3-2026", "$19.8M", "+31%"],
    ]

    md = table.to_markdown()
    assert "| Quarter | Revenue | Growth |" in md
    assert "| Q1-2026 | $12.4M | +18% |" in md

    records = table.to_dicts()
    assert len(records) == 3
    assert records[0] == {"Quarter": "Q1-2026", "Revenue": "$12.4M", "Growth": "+18%"}
    assert records[2]["Revenue"] == "$19.8M"


def test_real_pdf_rc4_encryption_and_decryption_roundtrip():
    doc = pdfrune.open()
    page = doc.new_page(width=612.0, height=792.0)
    page.insert_text(
        Point(72.0, 100.0),
        "Confidential Merger Blueprint 2026",
        fontsize=16.0,
    )

    encrypted_bytes = doc.to_bytes(
        encryption="rc4-128",
        user_pw="user_secret_pw",
        owner_pw="owner_master_pw",
    )

    # Verify the plaintext string is NOT visible in the raw encrypted PDF bytes!
    assert b"Confidential Merger Blueprint" not in encrypted_bytes

    # Open the encrypted PDF
    with pdfrune.open(encrypted_bytes) as enc_doc:
        assert enc_doc.is_encrypted is True
        assert enc_doc.needs_pass is True

        # Wrong password must fail
        assert enc_doc.authenticate("wrong_password") is False
        assert enc_doc.needs_pass is True

        # Correct user password must succeed and decrypt all streams/strings
        assert enc_doc.authenticate("user_secret_pw") is True
        assert enc_doc.needs_pass is False

        text = enc_doc[0].extract_text()
        assert "Confidential Merger Blueprint 2026" in text
