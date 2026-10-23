use pdfrune::parser::object::{PdfDict, PdfObject, PdfStream};
use pdfrune::{render_page, Document};
fn image_doc(filter: &str, data: Vec<u8>) -> Document {
    let mut d = Document::new();
    d.new_page(None, 16.0, 16.0).unwrap();
    let mut image = PdfDict::new();
    for (k, v) in [
        ("Type", "XObject"),
        ("Subtype", "Image"),
        ("ColorSpace", "DeviceRGB"),
        ("Filter", filter),
    ] {
        image.insert(
            k.as_bytes().to_vec(),
            PdfObject::Name(v.as_bytes().to_vec()),
        );
    }
    image.insert(b"Width".to_vec(), PdfObject::Int(16));
    image.insert(b"Height".to_vec(), PdfObject::Int(16));
    image.insert(b"BitsPerComponent".to_vec(), PdfObject::Int(8));
    let id = d.allocate_object_id();
    d.objects
        .insert(id.0, PdfObject::Stream(PdfStream::new(image, data)));
    let mut x = PdfDict::new();
    x.insert(b"Scan".to_vec(), PdfObject::Reference(id));
    d.pages[0]
        .resources
        .insert(b"XObject".to_vec(), PdfObject::Dict(x));
    d.append_page_content(0, b"q 16 0 0 16 0 0 cm /Scan Do Q")
        .unwrap();
    d
}
fn rgb(p: &pdfrune::Pixmap, x: usize, y: usize) -> [u8; 3] {
    let i = (y * p.width as usize + x) * 4;
    p.data[i..i + 3].try_into().unwrap()
}
#[test]
fn jpeg_scan_is_painted() {
    let d = image_doc("DCTDecode", include_bytes!("fixtures/scan.jpg").to_vec());
    let p = render_page(&d, 0, 72.0).unwrap();
    let a = rgb(&p, 3, 3);
    let b = rgb(&p, 3, 12);
    assert!(a[0] > 200 && a[2] < 60, "top={a:?}");
    assert!(b[2] > 200 && b[0] < 60, "bottom={b:?}");
}
#[test]
fn jpeg2000_scan_is_painted() {
    let d = image_doc("JPXDecode", include_bytes!("fixtures/scan.jp2").to_vec());
    let p = render_page(&d, 0, 72.0).unwrap();
    assert!(rgb(&p, 3, 3)[0] > 200);
    assert!(rgb(&p, 3, 12)[2] > 200);
    assert!(rgb(&p, 3, 3)[2] < 60);
}
#[test]
fn hidden_ocr_is_extractable_but_not_painted() {
    let mut d = Document::new();
    d.new_page(None, 100.0, 40.0).unwrap();
    d.append_page_content(0, b"BT /F1 16 Tf 3 Tr 10 20 Td (OCR HIDDEN) Tj ET")
        .unwrap();
    let spans = pdfrune::text::extract_page_spans(&d, 0).unwrap();
    assert!(spans.iter().any(|s| s.text.contains("OCR HIDDEN")));
    let p = render_page(&d, 0, 72.0).unwrap();
    assert!(p.data.chunks_exact(4).all(|px| px == [255, 255, 255, 255]));
}
#[test]
fn unsupported_scan_is_an_error() {
    let d = image_doc("UnknownDecode", vec![0; 16 * 16 * 3]);
    assert!(
        render_page(&d, 0, 72.0).is_err(),
        "unsupported images must not silently become white pages"
    );
}

#[test]
fn jbig2_soft_mask_composites_over_background() {
    let mut d = image_doc("JPXDecode", include_bytes!("fixtures/scan.jp2").to_vec());
    let mut dict = PdfDict::new();
    for (k, v) in [
        ("Type", "XObject"),
        ("Subtype", "Image"),
        ("ColorSpace", "DeviceGray"),
        ("Filter", "JBIG2Decode"),
    ] {
        dict.insert(
            k.as_bytes().to_vec(),
            PdfObject::Name(v.as_bytes().to_vec()),
        );
    }
    dict.insert(b"Width".to_vec(), PdfObject::Int(16));
    dict.insert(b"Height".to_vec(), PdfObject::Int(16));
    dict.insert(b"BitsPerComponent".to_vec(), PdfObject::Int(1));
    let mask = d.allocate_object_id();
    d.objects.insert(
        mask.0,
        PdfObject::Stream(PdfStream::new(
            dict,
            include_bytes!("fixtures/mask.jbig2").to_vec(),
        )),
    );
    let target = d.pages[0]
        .resources
        .get(b"XObject".as_slice())
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"Scan".as_slice())
        .unwrap()
        .as_reference()
        .unwrap();
    if let Some(PdfObject::Stream(s)) = d.objects.get_mut(&target.0) {
        s.dict.insert(b"SMask".to_vec(), PdfObject::Reference(mask));
    }
    let p = render_page(&d, 0, 72.0).unwrap();
    assert_eq!(
        rgb(&p, 12, 3),
        [255, 255, 255],
        "black mask must be transparent"
    );
    assert!(
        rgb(&p, 3, 3)[0] > 200 && rgb(&p, 3, 3)[2] < 60,
        "white mask must retain scan"
    );
}

#[test]
fn render_mode_is_saved_and_restored() {
    let mut d = Document::new();
    d.new_page(None, 100.0, 40.0).unwrap();
    d.append_page_content(
        0,
        b"q BT /F1 12 Tf 3 Tr 10 20 Td (hidden) Tj ET Q BT /F1 12 Tf 10 10 Td (visible) Tj ET",
    )
    .unwrap();
    let spans = pdfrune::text::extract_page_spans(&d, 0).unwrap();
    assert_eq!(spans[0].render_mode, 3);
    assert_eq!(spans[1].render_mode, 0);
    assert!(render_page(&d, 0, 72.0)
        .unwrap()
        .data
        .chunks_exact(4)
        .any(|px| px[0] < 100));
}

#[test]
fn image_affine_mirroring_keeps_row_orientation() {
    let mut d = image_doc("DCTDecode", include_bytes!("fixtures/scan.jpg").to_vec());
    // Paint an additional vertically mirrored image over the original.
    d.append_page_content(0, b"q 16 0 0 -16 0 16 cm /Scan Do Q")
        .unwrap();
    let p = render_page(&d, 0, 72.0).unwrap();
    assert!(rgb(&p, 3, 3)[2] > 200);
    assert!(rgb(&p, 3, 12)[0] > 200);
}

#[test]
fn packed_grayscale_decode_respects_row_padding_and_inversion() {
    let mut d = image_doc("FlateDecode", vec![]);
    let target = d.pages[0]
        .resources
        .get(b"XObject".as_slice())
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"Scan".as_slice())
        .unwrap()
        .as_reference()
        .unwrap();
    if let Some(PdfObject::Stream(s)) = d.objects.get_mut(&target.0) {
        s.dict.remove(b"Filter".as_slice());
        s.dict.insert(
            b"ColorSpace".to_vec(),
            PdfObject::Name(b"DeviceGray".to_vec()),
        );
        s.dict.insert(b"Width".to_vec(), PdfObject::Int(3));
        s.dict.insert(b"Height".to_vec(), PdfObject::Int(2));
        s.dict
            .insert(b"BitsPerComponent".to_vec(), PdfObject::Int(1));
        s.dict.insert(
            b"Decode".to_vec(),
            PdfObject::Array(vec![PdfObject::Int(1), PdfObject::Int(0)]),
        );
        s.data = vec![0b10100000, 0b01000000];
    }
    let stm = d.get_object(target).unwrap().as_stream().unwrap();
    let (w, h, rgba) = pdfrune::render::image::decode_image_xobject(&d, stm).unwrap();
    assert_eq!((w, h), (3, 2));
    assert_eq!(
        rgba.chunks_exact(4).map(|px| px[0]).collect::<Vec<_>>(),
        [0, 255, 0, 255, 0, 255]
    );
}
