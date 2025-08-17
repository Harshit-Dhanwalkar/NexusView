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

pub mod pdf_utils {
    use pdf::error::PdfError;
    use pdf::file::FileOptions;
    use pdf::object::*;
    use pdf::object::*;
    use pdf::primitive::PdfString;
    use pdf_extract::OutputError;
    use pdf_extract::content::Operation;
    use std::fmt;
    use std::path::Path;

    #[derive(Debug)]
    pub enum PdfUtilsError {
        Io(std::io::Error),
        PdfExtract(pdf_extract::Error),
        Custom(String),
    }

    impl fmt::Display for PdfUtilsError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                PdfUtilsError::Io(e) => write!(f, "IO error: {}", e),
                PdfUtilsError::PdfExtract(e) => write!(f, "PDF extract error: {}", e),
                PdfUtilsError::Custom(s) => write!(f, "{}", s),
            }
        }
    }

    impl std::error::Error for PdfUtilsError {}

    impl From<std::io::Error> for PdfUtilsError {
        fn from(err: std::io::Error) -> Self {
            PdfUtilsError::Io(err)
        }
    }

    impl From<pdf_extract::Error> for PdfUtilsError {
        fn from(err: pdf_extract::Error) -> Self {
            PdfUtilsError::PdfExtract(err)
        }
    }

    impl From<String> for PdfUtilsError {
        fn from(err: String) -> Self {
            PdfUtilsError::Custom(err)
        }
    }

    impl From<PdfUtilsError> for OutputError {
        fn from(err: PdfUtilsError) -> Self {
            match err {
                PdfUtilsError::Io(e) => OutputError::from(e),
                PdfUtilsError::PdfExtract(e) => OutputError::from(e),
                PdfUtilsError::Custom(s) => {
                    OutputError::from(std::io::Error::new(std::io::ErrorKind::Other, s))
                }
            }
        }
    }

    pub struct TextBlock {
        pub text: String,
        pub page: usize,
        pub x: f32,
        pub y: f32,
        pub width: f32,
        pub height: f32,
    }

    pub fn extract_text_with_layout(path: &Path) -> Result<Vec<TextBlock>, PdfError> {
        let data = std::fs::read(path)?;
        let file = FileOptions::cached().load(&data[..])?;
        let mut blocks = Vec::new();

        for (page_num, page) in file.pages().enumerate() {
            let page = page?;
            if let Some(content) = &page.contents {
                // let content = content.decode::<Vec<u8>>()?;
                // let content_str = String::from_utf8_lossy(&content);

                // Simple text extraction - replace with proper PDF text extraction
                blocks.push(TextBlock {
                    text: format!("Page {}", page_num + 1),
                    page: page_num,
                    x: 0.0,
                    y: 0.0,
                    width: 595.0,  // Default A4 width
                    height: 842.0, // Default A4 height
                });
            }
        }

        Ok(blocks)
    }
}
