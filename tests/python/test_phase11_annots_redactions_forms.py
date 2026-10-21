import pdfrune
from pdfrune import Point, Rect


def test_annotations_add_list_delete():
    doc = pdfrune.open()
    page = doc.new_page(width=612.0, height=792.0)
    assert page.annots() == []

    a1 = page.add_highlight_annot(Rect(72.0, 100.0, 250.0, 120.0))
    a2 = page.add_text_annot(Point(50.0, 50.0), "Review this paragraph")
    a3 = page.add_rect_annot(Rect(72.0, 150.0, 300.0, 220.0))

    assert a1["type"] == "Highlight"
    assert a2["type"] == "Text"
    assert a2["contents"] == "Review this paragraph"
    assert a3["type"] == "Square"

    annots = page.annots()
    assert len(annots) == 3

    # Delete the highlight annotation
    page.delete_annot(a1)
    remaining = page.annots()
    assert len(remaining) == 2
    assert [a["type"] for a in remaining] == ["Text", "Square"]

    # Roundtrip serialize & reopen
    raw = doc.to_bytes()
    with pdfrune.open(raw) as reopened:
        r_annots = reopened[0].annots()
        assert len(r_annots) == 2
        assert r_annots[0]["contents"] == "Review this paragraph"


def test_true_pdf_redaction_scrubs_underlying_text():
    doc = pdfrune.open()
    page = doc.new_page(width=612.0, height=792.0)
    page.insert_text(Point(72.0, 100.0), "Public Header Information", fontsize=16.0)
    page.insert_text(Point(72.0, 160.0), "SSN: 999-00-1234", fontsize=14.0)
    page.insert_text(Point(72.0, 220.0), "Unredacted Footer Line", fontsize=14.0)

    assert "999-00-1234" in page.extract_text()
    secret_rects = page.search("999-00-1234")
    assert len(secret_rects) == 1

    # Add a redaction annotation over the secret SSN line
    page.add_redact_annot(
        Rect(65.0, 140.0, 300.0, 175.0),
        text="[REDACTED]",
        fill=(0.0, 0.0, 0.0),
    )
    assert any(a["type"] == "Redact" for a in page.annots())

    # Apply redactions permanently
    removed_count = page.apply_redactions()
    assert removed_count >= 1

    # Verify the Redact annotation is gone and the secret text is completely scrubbed
    assert not any(a["type"] == "Redact" for a in page.annots())
    extracted_after = page.extract_text()
    assert "999-00-1234" not in extracted_after
    assert "Public Header Information" in extracted_after
    assert "Unredacted Footer Line" in extracted_after
    assert "[REDACTED]" in extracted_after

    # Verify even after saving and reopening, the redacted text does not exist in the PDF
    raw = doc.to_bytes()
    assert b"999-00-1234" not in raw
    with pdfrune.open(raw) as reopened:
        assert "999-00-1234" not in reopened[0].extract_text()
        assert reopened[0].search("999-00-1234") == []


def test_interactive_acroform_widgets():
    doc = pdfrune.open()
    page = doc.new_page(width=612.0, height=792.0)
    assert page.widgets() == []

    w1 = page.add_widget(
        {
            "field_name": "applicant_name",
            "field_type": "text",
            "rect": Rect(100.0, 100.0, 300.0, 125.0),
            "field_value": "Gwang Jin",
        }
    )
    w2 = page.add_widget(
        {
            "field_name": "terms_accepted",
            "field_type": "checkbox",
            "rect": Rect(100.0, 140.0, 120.0, 160.0),
            "field_value": "Yes",
        }
    )
    assert w1["field_name"] == "applicant_name"
    assert w2["field_type"] == "checkbox"

    widgets = page.widgets()
    assert len(widgets) == 2
    assert widgets[0]["field_value"] == "Gwang Jin"
    assert widgets[1]["field_value"] == "Yes"

    # Update field value in place
    assert page.set_field_value("applicant_name", "Updated Applicant")
    assert page.widgets()[0]["field_value"] == "Updated Applicant"

    # Roundtrip serialize & reopen
    raw = doc.to_bytes()
    with pdfrune.open(raw) as reopened:
        rw = reopened[0].widgets()
        assert len(rw) == 2
        assert rw[0]["field_name"] == "applicant_name"
        assert rw[0]["field_value"] == "Updated Applicant"
