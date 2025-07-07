// src/file_scan.rs
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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

    pub fn scan_directory(&mut self, path: &Path) -> Result<(), String> {
        if !path.is_dir() {
            return Err(format!("Path is not a directory: {:?}", path));
        }

        self.files.clear();
        self.tags.clear();
        self.images.clear();

        self.traverse_directory(path)?;

        let mut resolved_files = HashMap::new();
        for (file_path, links) in &self.files {
            let mut resolved_links_for_file = Vec::new();
            for link in links {
                let resolved_link = if link.is_relative() {
                    self.root_path.join(link)
                } else {
                    link.clone()
                };
                resolved_links_for_file.push(resolved_link);
            }
            resolved_files.insert(file_path.clone(), resolved_links_for_file);
        }
        self.files = resolved_files;

        Ok(())
    }

    fn traverse_directory(&mut self, path: &Path) -> Result<(), String> {
        let image_extensions = ["png", "jpg", "jpeg", "gif", "bmp", "webp"];
        let tag_re = Regex::new(r"#(\w+)").unwrap();
        let link_re = Regex::new(r"\\[\\[([^\\]]+)\\]\\]").unwrap();

        for entry_result in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry_result.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.is_dir() {
                self.traverse_directory(&path)?;
            } else if path.is_file() {
                if let Some(extension) = path.extension() {
                    let ext_str = extension.to_string_lossy().to_lowercase();

                    if image_extensions.contains(&ext_str.as_str()) {
                        self.images.push(path.clone());
                    } else if ext_str == "md" || ext_str == "txt" {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let links = link_re
                                .captures_iter(&content)
                                .filter_map(|cap| cap.get(1))
                                .map(|m| PathBuf::from(m.as_str()))
                                .collect();
                            self.files.insert(path.clone(), links);

                            let found_tags: Vec<String> = tag_re
                                .captures_iter(&content)
                                .filter_map(|cap| cap.get(1))
                                .map(|m| m.as_str().to_string())
                                .collect();
                            if !found_tags.is_empty() {
                                self.tags.insert(path.clone(), found_tags);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
