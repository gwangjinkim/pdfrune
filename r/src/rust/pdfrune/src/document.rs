use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::error::{Result, PdfruneError};
use crate::geometry::Rect;
use crate::page::{normalize_rotation, Page};
use crate::parser::lexer::Lexer;
use crate::parser::object::{ObjectId, PdfDict, PdfDictExt, PdfObject};
use crate::parser::trailer::{extract_metadata_map, unpack_object_stream};
use crate::parser::xref::{parse_xref_and_trailer, XrefEntry};

#[derive(Debug, Clone, Default)]
struct InheritedPageAttrs {
    mediabox: Option<Rect>,
    cropbox: Option<Rect>,
    rotate: Option<i32>,
    resources: Option<PdfDict>,
}

/// Core in-memory representation of a PDF Document.
#[derive(Debug, Clone)]
pub struct Document {
    pub pdf_version: String,
    pub objects: BTreeMap<u32, PdfObject>,
    pub trailer: PdfDict,
    pub pages: Vec<Page>,
    pub is_encrypted: bool,
    pub is_authenticated: bool,
}

impl Document {
    /// Opens a PDF document from a filesystem path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        Self::from_bytes(&bytes)
    }

    /// Creates a new, empty PDF-1.7 document ready for page insertion or creation.
    pub fn new() -> Self {
        let mut objects = BTreeMap::new();
        let mut catalog = PdfDict::new();
        catalog.insert(b"Type".to_vec(), PdfObject::Name(b"Catalog".to_vec()));
        catalog.insert(b"Pages".to_vec(), PdfObject::Reference((2, 0)));
        objects.insert(1, PdfObject::Dict(catalog));

        let mut pages_dict = PdfDict::new();
        pages_dict.insert(b"Type".to_vec(), PdfObject::Name(b"Pages".to_vec()));
        pages_dict.insert(b"Kids".to_vec(), PdfObject::Array(Vec::new()));
        pages_dict.insert(b"Count".to_vec(), PdfObject::Int(0));
        objects.insert(2, PdfObject::Dict(pages_dict));

        let mut info = PdfDict::new();
        info.insert(
            b"Producer".to_vec(),
            PdfObject::String(b"pdfrune 0.1.0".to_vec()),
        );
        objects.insert(3, PdfObject::Dict(info));

        let mut trailer = PdfDict::new();
        trailer.insert(b"Root".to_vec(), PdfObject::Reference((1, 0)));
        trailer.insert(b"Info".to_vec(), PdfObject::Reference((3, 0)));

        Self {
            pdf_version: "1.7".to_string(),
            objects,
            trailer,
            pages: Vec::new(),
            is_encrypted: false,
            is_authenticated: true,
        }
    }

    /// Parses a PDF document from an in-memory byte slice.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let xref_table = parse_xref_and_trailer(data)?;
        let mut objects = BTreeMap::new();
        let mut compressed_deferred = Vec::new();

        // Pass 1: Parse all uncompressed indirect objects
        for (&obj_num, &entry) in &xref_table.entries {
            match entry {
                XrefEntry::Uncompressed { offset, .. } => {
                    if offset < data.len() {
                        let mut lexer = Lexer::with_offset(data, offset);
                        if let Ok((_, obj)) = lexer.parse_indirect_object() {
                            objects.insert(obj_num, obj);
                        }
                    }
                }
                XrefEntry::Compressed {
                    stream_obj_num,
                    index,
                } => {
                    compressed_deferred.push((obj_num, stream_obj_num, index));
                }
            }
        }

        // Resolve indirect /Length for streams that needed it during initial lexing
        // (Our lexer already falls back to SIMD `endstream` search when /Length is an indirect ref)

        // Pass 2: Unpack compressed object streams (/ObjStm)
        let mut unpacked_cache: BTreeMap<u32, BTreeMap<u32, PdfObject>> = BTreeMap::new();
        for (obj_num, stm_num, _idx) in compressed_deferred {
            if let std::collections::btree_map::Entry::Vacant(e) = unpacked_cache.entry(stm_num) {
                if let Some(stm_obj) = objects.get(&stm_num).and_then(PdfObject::as_stream) {
                    if let Ok(map) = unpack_object_stream(stm_obj) {
                        e.insert(map);
                    }
                }
            }
            if let Some(obj) = unpacked_cache.get(&stm_num).and_then(|m| m.get(&obj_num)) {
                objects.insert(obj_num, obj.clone());
            }
        }

        let is_encrypted = xref_table.trailer.contains_key(b"Encrypt".as_slice());

        let mut doc = Self {
            pdf_version: xref_table.pdf_version,
            objects,
            trailer: xref_table.trailer,
            pages: Vec::new(),
            is_encrypted,
            is_authenticated: !is_encrypted,
        };

        doc.rebuild_page_tree()?;
        Ok(doc)
    }

    /// Allocates and reserves the next available object number in `self.objects`.
    pub fn allocate_object_id(&mut self) -> ObjectId {
        let next_num = self.objects.keys().next_back().copied().unwrap_or(0) + 1;
        self.objects.insert(next_num, PdfObject::Null);
        (next_num, 0)
    }

    /// Looks up an indirect object by its `ObjectId` or object number.
    pub fn get_object(&self, id: ObjectId) -> Option<&PdfObject> {
        self.objects.get(&id.0)
    }

    /// Resolves a `PdfObject` if it is an indirect `Reference`, following reference chains safely.
    pub fn resolve<'a>(&'a self, obj: &'a PdfObject) -> &'a PdfObject {
        let mut current = obj;
        for _ in 0..16 {
            if let PdfObject::Reference(id) = current {
                if let Some(target) = self.objects.get(&id.0) {
                    current = target;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        current
    }

    /// Resolves a dictionary entry by key, dereferencing indirect references automatically.
    pub fn resolve_dict_entry<'a>(
        &'a self,
        dict: &'a PdfDict,
        key: &[u8],
    ) -> Option<&'a PdfObject> {
        dict.get(key).map(|obj| self.resolve(obj))
    }

    /// Returns the number of pages in the document.
    #[inline]
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Returns a reference to the `Page` at `index` (0-based).
    pub fn page(&self, index: usize) -> Result<&Page> {
        self.pages.get(index).ok_or(PdfruneError::PageOutOfBounds {
            index,
            count: self.pages.len(),
        })
    }

    /// Returns the document metadata dictionary (`title`, `author`, `subject`, `keywords`, etc.).
    pub fn metadata(&self) -> BTreeMap<String, String> {
        let info_dict = self
            .resolve_dict_entry(&self.trailer, b"Info")
            .and_then(PdfObject::as_dict);
        extract_metadata_map(info_dict, &self.pdf_version)
    }

    /// Updates or sets document metadata fields in `/Info`.
    pub fn set_metadata(&mut self, updates: &BTreeMap<String, String>) {
        let info_id = if let Some(id) = self.trailer.get_ref(b"Info") {
            id
        } else {
            let new_id = self.allocate_object_id();
            self.objects
                .insert(new_id.0, PdfObject::Dict(PdfDict::new()));
            self.trailer
                .insert(b"Info".to_vec(), PdfObject::Reference(new_id));
            new_id
        };

        self.objects
            .entry(info_id.0)
            .or_insert_with(|| PdfObject::Dict(PdfDict::new()));

        if let Some(PdfObject::Dict(info_dict)) = self.objects.get_mut(&info_id.0) {
            let mapping: &[(&str, &[u8])] = &[
                ("title", b"Title"),
                ("author", b"Author"),
                ("subject", b"Subject"),
                ("keywords", b"Keywords"),
                ("creator", b"Creator"),
                ("producer", b"Producer"),
                ("creationDate", b"CreationDate"),
                ("modDate", b"ModDate"),
            ];
            for (k_str, pdf_key) in mapping {
                if let Some(val) = updates.get(*k_str).or_else(|| {
                    let cap = std::str::from_utf8(pdf_key).unwrap_or("");
                    updates.get(cap)
                }) {
                    info_dict.insert(pdf_key.to_vec(), PdfObject::String(val.as_bytes().to_vec()));
                }
            }
        }
    }

    /// Authenticates an encrypted document with a user/owner password and decrypts objects in-place.
    pub fn authenticate(&mut self, password: &str) -> Result<bool> {
        crate::crypto::authenticate_and_decrypt(self, password)
    }

    /// Traverses the `/Catalog -> /Pages` tree and populates `self.pages`.
    pub fn rebuild_page_tree(&mut self) -> Result<()> {
        let root_obj = self
            .resolve_dict_entry(&self.trailer, b"Root")
            .cloned()
            .or_else(|| {
                // Find any /Type /Catalog in objects if /Root wasn't in trailer
                self.objects
                    .values()
                    .find(|o| {
                        o.as_dict()
                            .is_some_and(|d| d.get_name(b"Type") == Some(b"Catalog"))
                    })
                    .cloned()
            });

        let mut pages = Vec::new();
        let mut visited = BTreeSet::new();

        if let Some(catalog) = root_obj.as_ref().and_then(PdfObject::as_dict) {
            if let Some(pages_ref) = catalog.get(b"Pages".as_slice()) {
                let pages_id = pages_ref.as_reference().unwrap_or((0, 0));
                let pages_node = self.resolve(pages_ref).clone();
                self.collect_pages_recursive(
                    pages_id,
                    &pages_node,
                    InheritedPageAttrs::default(),
                    &mut visited,
                    &mut pages,
                );
            }
        }

        // Fallback: if /Pages tree was broken or empty, scan for all `/Type /Page` objects in order
        if pages.is_empty() {
            for (&obj_num, obj) in &self.objects {
                if let Some(dict) = obj.as_dict() {
                    if dict.get_name(b"Type") == Some(b"Page") {
                        let page = self.build_page_from_dict(
                            pages.len(),
                            (obj_num, 0),
                            dict,
                            &InheritedPageAttrs::default(),
                        );
                        pages.push(page);
                    }
                }
            }
        }

        self.pages = pages;
        Ok(())
    }

    fn collect_pages_recursive(
        &self,
        node_id: ObjectId,
        node_obj: &PdfObject,
        mut inherited: InheritedPageAttrs,
        visited: &mut BTreeSet<u32>,
        out_pages: &mut Vec<Page>,
    ) {
        if node_id.0 != 0 && !visited.insert(node_id.0) {
            return; // Prevent infinite recursion on circular /Pages trees
        }

        let Some(dict) = node_obj.as_dict() else {
            return;
        };

        if let Some(mb) = self
            .resolve_dict_entry(dict, b"MediaBox")
            .and_then(PdfObject::as_rect)
        {
            inherited.mediabox = Some(Rect::from_array(mb));
        }
        if let Some(cb) = self
            .resolve_dict_entry(dict, b"CropBox")
            .and_then(PdfObject::as_rect)
        {
            inherited.cropbox = Some(Rect::from_array(cb));
        }
        if let Some(rot) = self
            .resolve_dict_entry(dict, b"Rotate")
            .and_then(PdfObject::as_i64)
        {
            inherited.rotate = Some(normalize_rotation(rot));
        }
        if let Some(res) = self
            .resolve_dict_entry(dict, b"Resources")
            .and_then(PdfObject::as_dict)
        {
            inherited.resources = Some(res.clone());
        }

        let node_type = dict.get_name(b"Type");
        if node_type == Some(b"Pages") || dict.contains_key(b"Kids".as_slice()) {
            if let Some(kids) = self
                .resolve_dict_entry(dict, b"Kids")
                .and_then(PdfObject::as_array)
            {
                for kid in kids {
                    let kid_id = kid.as_reference().unwrap_or((0, 0));
                    let resolved_kid = self.resolve(kid);
                    self.collect_pages_recursive(
                        kid_id,
                        resolved_kid,
                        inherited.clone(),
                        visited,
                        out_pages,
                    );
                }
            }
        } else if node_type == Some(b"Page") || dict.contains_key(b"Contents".as_slice()) {
            let page = self.build_page_from_dict(out_pages.len(), node_id, dict, &inherited);
            out_pages.push(page);
        }
    }

    fn build_page_from_dict(
        &self,
        index: usize,
        object_id: ObjectId,
        dict: &PdfDict,
        inherited: &InheritedPageAttrs,
    ) -> Page {
        let mediabox = self
            .resolve_dict_entry(dict, b"MediaBox")
            .and_then(PdfObject::as_rect)
            .map(Rect::from_array)
            .or(inherited.mediabox)
            .unwrap_or_default();

        let cropbox = self
            .resolve_dict_entry(dict, b"CropBox")
            .and_then(PdfObject::as_rect)
            .map(Rect::from_array)
            .or(inherited.cropbox)
            .unwrap_or(mediabox);

        let bleedbox = self
            .resolve_dict_entry(dict, b"BleedBox")
            .and_then(PdfObject::as_rect)
            .map(Rect::from_array)
            .unwrap_or(cropbox);

        let trimbox = self
            .resolve_dict_entry(dict, b"TrimBox")
            .and_then(PdfObject::as_rect)
            .map(Rect::from_array)
            .unwrap_or(cropbox);

        let artbox = self
            .resolve_dict_entry(dict, b"ArtBox")
            .and_then(PdfObject::as_rect)
            .map(Rect::from_array)
            .unwrap_or(cropbox);

        let rotation = self
            .resolve_dict_entry(dict, b"Rotate")
            .and_then(PdfObject::as_i64)
            .map(normalize_rotation)
            .or(inherited.rotate)
            .unwrap_or(0);

        let mut resources = inherited.resources.clone().unwrap_or_default();
        if let Some(page_res) = self
            .resolve_dict_entry(dict, b"Resources")
            .and_then(PdfObject::as_dict)
        {
            for (k, v) in page_res {
                resources.insert(k.clone(), v.clone());
            }
        }

        Page {
            index,
            object_id,
            mediabox,
            cropbox,
            bleedbox,
            trimbox,
            artbox,
            rotation,
            resources,
        }
    }

    /// Concatenates and decompresses all `/Contents` streams for the given 0-based `page_index`.
    pub fn page_content_bytes(&self, page_index: usize) -> Result<Vec<u8>> {
        let page = self.page(page_index)?;
        let Some(page_dict) = self
            .objects
            .get(&page.object_id.0)
            .and_then(PdfObject::as_dict)
        else {
            return Ok(Vec::new());
        };

        let Some(contents_obj) = self.resolve_dict_entry(page_dict, b"Contents") else {
            return Ok(Vec::new());
        };

        let mut combined = Vec::new();
        match contents_obj {
            PdfObject::Stream(stm) => {
                if let Ok(decoded) = stm.decode_data() {
                    combined.extend_from_slice(&decoded);
                } else {
                    combined.extend_from_slice(&stm.data);
                }
            }
            PdfObject::Array(arr) => {
                for item in arr {
                    let resolved = self.resolve(item);
                    if let Some(stm) = resolved.as_stream() {
                        if !combined.is_empty() {
                            combined.push(b'\n');
                        }
                        if let Ok(decoded) = stm.decode_data() {
                            combined.extend_from_slice(&decoded);
                        } else {
                            combined.extend_from_slice(&stm.data);
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(combined)
    }

    /// Sets the rotation (in degrees clockwise) of the page at `page_index`.
    pub fn set_page_rotation(&mut self, page_index: usize, rotation: i64) -> Result<()> {
        let norm = normalize_rotation(rotation);
        let obj_id = self.page(page_index)?.object_id;
        if let Some(page) = self.pages.get_mut(page_index) {
            page.rotation = norm;
        }
        if let Some(dict) = self
            .objects
            .get_mut(&obj_id.0)
            .and_then(PdfObject::as_dict_mut)
        {
            dict.insert(b"Rotate".to_vec(), PdfObject::Int(norm as i64));
        }
        Ok(())
    }

    /// Sets the `/MediaBox` of the page at `page_index`.
    pub fn set_page_mediabox(&mut self, page_index: usize, rect: Rect) -> Result<()> {
        let norm = rect.normalize();
        let obj_id = self.page(page_index)?.object_id;
        if let Some(page) = self.pages.get_mut(page_index) {
            page.mediabox = norm;
        }
        if let Some(dict) = self
            .objects
            .get_mut(&obj_id.0)
            .and_then(PdfObject::as_dict_mut)
        {
            dict.insert(
                b"MediaBox".to_vec(),
                PdfObject::Array(vec![
                    PdfObject::Real(norm.x0),
                    PdfObject::Real(norm.y0),
                    PdfObject::Real(norm.x1),
                    PdfObject::Real(norm.y1),
                ]),
            );
        }
        Ok(())
    }

    /// Sets the `/CropBox` of the page at `page_index`.
    pub fn set_page_cropbox(&mut self, page_index: usize, rect: Rect) -> Result<()> {
        let norm = rect.normalize();
        let obj_id = self.page(page_index)?.object_id;
        if let Some(page) = self.pages.get_mut(page_index) {
            page.cropbox = norm;
        }
        if let Some(dict) = self
            .objects
            .get_mut(&obj_id.0)
            .and_then(PdfObject::as_dict_mut)
        {
            dict.insert(
                b"CropBox".to_vec(),
                PdfObject::Array(vec![
                    PdfObject::Real(norm.x0),
                    PdfObject::Real(norm.y0),
                    PdfObject::Real(norm.x1),
                    PdfObject::Real(norm.y1),
                ]),
            );
        }
        Ok(())
    }

    /// Extracts plain text in reading order for the page at `page_index`.
    pub fn extract_page_text(&self, page_index: usize) -> Result<String> {
        let layout = self.page_layout(page_index)?;
        Ok(layout.plain_text())
    }

    /// Computes the structured `PageLayout` (blocks, lines, words, bounding boxes) for `page_index`.
    pub fn page_layout(&self, page_index: usize) -> Result<crate::text::PageLayout> {
        crate::text::PageLayout::from_document(self, page_index)
    }

    /// Searches for `needle` on `page_index` and returns matching bounding boxes.
    pub fn search_page(&self, page_index: usize, needle: &str) -> Result<Vec<Rect>> {
        let layout = self.page_layout(page_index)?;
        Ok(layout.search(needle))
    }

    /// Serializes this document to PDF bytes.
    pub fn to_bytes(&self, deflate: bool) -> Result<Vec<u8>> {
        crate::mutate::serialize_document(self, deflate)
    }

    /// Serializes this document with optional RC4-128 user/owner password encryption.
    pub fn to_bytes_encrypted(
        &self,
        deflate: bool,
        user_pw: Option<&str>,
        owner_pw: Option<&str>,
    ) -> Result<Vec<u8>> {
        crate::mutate::serialize_document_encrypted(self, deflate, user_pw, owner_pw)
    }

    /// Saves this document to a filesystem path.
    pub fn save<P: AsRef<Path>>(&self, path: P, deflate: bool) -> Result<()> {
        crate::mutate::save_document(self, path, deflate)
    }

    /// Saves this document to a filesystem path with optional user/owner password encryption.
    pub fn save_encrypted<P: AsRef<Path>>(
        &self,
        path: P,
        deflate: bool,
        user_pw: Option<&str>,
        owner_pw: Option<&str>,
    ) -> Result<()> {
        let bytes = self.to_bytes_encrypted(deflate, user_pw, owner_pw)?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Detects structured tables on `page_index`.
    pub fn find_page_tables(
        &self,
        page_index: usize,
    ) -> Result<Vec<crate::advanced::DetectedTable>> {
        crate::advanced::find_page_tables(self, page_index)
    }

    /// Rasterizes `page_index` at `dpi` into a `Pixmap` (RGBA8888).
    pub fn render_page(&self, page_index: usize, dpi: f64) -> Result<crate::render::Pixmap> {
        crate::render::render_page(self, page_index, dpi)
    }

    /// Rasterizes all pages in parallel via `rayon` at `dpi` into `Pixmap`s.
    pub fn render_all_pages(&self, dpi: f64) -> Result<Vec<crate::render::Pixmap>> {
        use rayon::prelude::*;
        (0..self.page_count())
            .into_par_iter()
            .map(|idx| self.render_page(idx, dpi))
            .collect()
    }

    /// Exports `page_index` to a vector SVG string.
    pub fn render_page_svg(&self, page_index: usize) -> Result<String> {
        crate::render::render_page_svg(self, page_index)
    }

    /// Exports `page_index` to structured Markdown.
    pub fn page_to_markdown(&self, page_index: usize) -> Result<String> {
        crate::advanced::page_to_markdown(self, page_index)
    }

    /// Exports the entire document to structured Markdown.
    pub fn to_markdown(&self) -> Result<String> {
        crate::advanced::doc_to_markdown(self)
    }

    /// Exports `page_index` to semantic HTML.
    pub fn page_to_html(&self, page_index: usize) -> Result<String> {
        crate::advanced::page_to_html(self, page_index)
    }

    /// Exports the entire document to semantic HTML.
    pub fn to_html(&self) -> Result<String> {
        crate::advanced::doc_to_html(self)
    }

    /// Generates semantic RAG chunks with bounding box coordinates across the document.
    pub fn to_rag_chunks(&self, max_chars: usize) -> Result<Vec<crate::advanced::RagChunk>> {
        crate::advanced::doc_to_rag_chunks(self, max_chars)
    }

    /// Searches across all pages in parallel using SIMD substring or regex matching.
    pub fn search_all(&self, query: &str, use_regex: bool) -> Result<Vec<(usize, Vec<Rect>)>> {
        crate::advanced::search_document(self, query, use_regex)
    }

    /// Extracts the Table of Contents (`/Outlines`) hierarchy.
    pub fn get_toc(&self) -> Vec<crate::nav::TocEntry> {
        crate::nav::get_toc(self)
    }

    /// Sets the Table of Contents (`/Outlines`) hierarchy.
    pub fn set_toc(&mut self, entries: &[crate::nav::TocEntry]) -> Result<usize> {
        crate::nav::set_toc(self, entries)
    }

    /// Returns all hyperlink annotations on `page_index`.
    pub fn get_page_links(&self, page_index: usize) -> Result<Vec<crate::nav::LinkEntry>> {
        crate::nav::get_page_links(self, page_index)
    }

    /// Inserts a hyperlink annotation on `page_index`.
    pub fn insert_page_link(
        &mut self,
        page_index: usize,
        from: Rect,
        uri: Option<&str>,
        target_page: Option<usize>,
    ) -> Result<u32> {
        crate::nav::insert_page_link(self, page_index, from, uri, target_page)
    }

    /// Deletes a hyperlink annotation from `page_index`.
    pub fn delete_page_link(
        &mut self,
        page_index: usize,
        xref: Option<u32>,
        from: Option<Rect>,
    ) -> Result<()> {
        crate::nav::delete_page_link(self, page_index, xref, from)
    }

    /// Lists all `/Image` XObjects referenced on `page_index`.
    pub fn get_page_images(&self, page_index: usize) -> Result<Vec<crate::render::PageImageInfo>> {
        crate::render::get_page_images(self, page_index)
    }

    /// Extracts an `/Image` XObject by its indirect object number (`xref`).
    pub fn extract_image(&self, xref: u32) -> Result<crate::render::ExtractedImage> {
        crate::render::extract_image(self, xref)
    }

    /// Inserts a PNG image byte slice onto `page_index` at `rect`.
    pub fn insert_image_bytes(
        &mut self,
        page_index: usize,
        rect: Rect,
        image_bytes: &[u8],
    ) -> Result<u32> {
        crate::render::insert_image_bytes(self, page_index, rect, image_bytes)
    }

    /// Inserts a `Pixmap` onto `page_index` at `rect`.
    pub fn insert_image_pixmap(
        &mut self,
        page_index: usize,
        rect: Rect,
        pixmap: &crate::render::Pixmap,
    ) -> Result<u32> {
        crate::render::insert_image_pixmap(self, page_index, rect, pixmap)
    }

    /// Lists all annotations on `page_index`.
    pub fn get_page_annots(&self, page_index: usize) -> Result<Vec<crate::annot::AnnotInfo>> {
        crate::annot::get_page_annots(self, page_index)
    }

    /// Adds an annotation to `page_index`.
    pub fn add_page_annot(
        &mut self,
        page_index: usize,
        subtype: &str,
        rect: Rect,
        contents: Option<&str>,
        color: [f64; 3],
        overlay_text: Option<&str>,
    ) -> Result<crate::annot::AnnotInfo> {
        crate::annot::add_page_annot(
            self,
            page_index,
            subtype,
            rect,
            contents,
            color,
            overlay_text,
        )
    }

    /// Deletes an annotation by `xref` from `page_index`.
    pub fn delete_page_annot(&mut self, page_index: usize, xref: u32) -> Result<()> {
        crate::annot::delete_page_annot(self, page_index, xref)
    }

    /// Applies all `/Redact` annotations on `page_index`, scrubbing underlying text.
    pub fn apply_page_redactions(&mut self, page_index: usize) -> Result<usize> {
        crate::annot::apply_page_redactions(self, page_index)
    }

    /// Lists all AcroForm `/Widget` fields on `page_index`.
    pub fn get_page_widgets(&self, page_index: usize) -> Result<Vec<crate::annot::WidgetInfo>> {
        crate::annot::get_page_widgets(self, page_index)
    }

    /// Adds an interactive AcroForm `/Widget` field to `page_index`.
    pub fn add_page_widget(
        &mut self,
        page_index: usize,
        field_name: &str,
        field_type: &str,
        rect: Rect,
        field_value: &str,
    ) -> Result<crate::annot::WidgetInfo> {
        crate::annot::add_page_widget(self, page_index, field_name, field_type, rect, field_value)
    }

    /// Updates the value of an AcroForm `/Widget` field on `page_index`.
    pub fn set_page_field_value(
        &mut self,
        page_index: usize,
        field_name: &str,
        new_value: &str,
    ) -> Result<bool> {
        crate::annot::set_page_field_value(self, page_index, field_name, new_value)
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}
