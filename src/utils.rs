// src/util.rs
use std::path::Path;

pub fn is_image_path(path: &Path) -> bool {
    path.extension().map_or(false, |ext| {
        let ext = ext.to_string_lossy().to_lowercase();
        ["png", "jpg", "jpeg", "gif", "bmp", "webp", "ind"].contains(&ext.as_str())
    })
}

pub fn is_markdown_path(path: &Path) -> bool {
    path.extension().map_or(false, |ext| {
        ext.to_str().unwrap_or("").to_lowercase() == "md"
    })
}

pub fn is_code_path(path: &Path) -> bool {
    path.extension().map_or(false, |ext| {
        let ext_lower = ext.to_str().unwrap_or("").to_lowercase();
        ["rs", "py", "c", "cpp", "h", "js", "html", "css", "sh"].contains(&ext_lower.as_str())
    })
}

pub fn is_pdf_path(path: &Path) -> bool {
    path.extension().map_or(false, |ext| {
        ext.to_str().unwrap_or("").to_lowercase() == "pdf"
    })
}

pub fn rotate_vec2(vec: egui::Vec2, angle_radians: f32) -> egui::Vec2 {
    let cos_a = angle_radians.cos();
    let sin_a = angle_radians.sin();
    egui::vec2(vec.x * cos_a - vec.y * sin_a, vec.x * sin_a + vec.y * cos_a)
}
