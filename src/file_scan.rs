use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct FileScanner {
    root_path: PathBuf,
    pub files: HashMap<PathBuf, Vec<PathBuf>>,
    pub images: Vec<PathBuf>,
    pub tags: HashMap<PathBuf, Vec<String>>,
}

impl FileScanner {
    pub fn new(root_path: impl AsRef<Path>) -> Self {
        Self {
            root_path: root_path.as_ref().to_path_buf(),
            files: HashMap::new(),
            images: Vec::new(),
            tags: HashMap::new(),
        }
    }

    pub fn scan(&mut self) {
        let image_extensions = ["png", "jpg", "jpeg", "gif", "bmp", "webp"];
        let tag_re = Regex::new(r"#(\w+)").unwrap(); // Regex to find tags

        // First pass: collect all files
        let mut all_files = Vec::new();
        for entry in WalkDir::new(&self.root_path) {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() {
                    let path = entry.path().to_path_buf();

                    // Check if file is an image
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if image_extensions.contains(&ext.to_lowercase().as_str()) {
                            self.images.push(path.clone());
                            continue;
                        }
                    }
                    all_files.push(path);
                }
            }
        }

        // Second pass: find links and tags between files
        for file_path in all_files {
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                let re = regex::Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
                let links = re
                    .captures_iter(&content)
                    .filter_map(|cap| cap.get(1))
                    .map(|m| self.root_path.join(m.as_str()))
                    .collect();
                self.files.insert(file_path.clone(), links);

                // Tag extraction
                let found_tags: Vec<String> = tag_re
                    .captures_iter(&content)
                    .filter_map(|cap| cap.get(1))
                    .map(|m| m.as_str().to_string())
                    .collect();
                if !found_tags.is_empty() {
                    self.tags.insert(file_path, found_tags); // Store extracted tags
                }
            }
        }
    }
}
