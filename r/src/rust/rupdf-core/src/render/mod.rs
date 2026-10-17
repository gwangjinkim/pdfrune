pub mod engine;
pub mod image;
pub mod pixmap;
pub mod svg;

pub use engine::render_page;
pub use image::{
    decode_image_xobject_to_rgba, extract_image, get_page_images, insert_image_bytes,
    insert_image_pixmap, ExtractedImage, PageImageInfo,
};
pub use pixmap::Pixmap;
pub use svg::render_page_svg;
