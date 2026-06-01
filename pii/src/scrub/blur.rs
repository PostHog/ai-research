use base64::Engine;
use image::{imageops::FilterType, ImageEncoder};

const TARGET: u32 = 16;

pub fn blur_image_data_uri(s: &str) -> Option<String> {
    let rest = s.strip_prefix("data:")?;
    let (meta, payload) = rest.split_once(',')?;
    if !meta.contains("base64") || !meta.starts_with("image/") {
        return None;
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload.as_bytes())
        .ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    let (w, h) = (img.width().max(1), img.height().max(1));
    let scale = TARGET as f32 / w.max(h) as f32;
    let nw = ((w as f32 * scale).round() as u32).max(1);
    let nh = ((h as f32 * scale).round() as u32).max(1);
    let small = img.resize_exact(nw, nh, FilterType::Triangle);

    let rgba = small.to_rgba8();
    let mut out = Vec::with_capacity(256);
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(rgba.as_raw(), nw, nh, image::ExtendedColorType::Rgba8)
        .ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&out);
    let mut uri = String::with_capacity(32 + encoded.len());
    uri.push_str("data:image/png;base64,");
    uri.push_str(&encoded);
    Some(uri)
}

pub fn blur_media_src(s: &str) -> Option<String> {
    if s.starts_with("data:image/") {
        return blur_image_data_uri(s);
    }
    None
}
