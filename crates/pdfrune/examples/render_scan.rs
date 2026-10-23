use pdfrune::{render_page, Document};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let d = Document::open(&args[1]).unwrap();
    let p: usize = args[2].parse().unwrap();
    let pix = render_page(&d, p, 144.0).unwrap();
    pix.save_png(&args[3]).unwrap();
}
