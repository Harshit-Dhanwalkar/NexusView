// src/file_scan.rs
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

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

    pub fn scan_directory_with_progress(
        &mut self,
        path: &Path,
        progress_sender: Sender<(f32, String)>,
    ) -> Result<(), String> {
        if !path.is_dir() {
            return Err(format!("Path is not a directory: {:?}", path));
        }

        let entries: Vec<_> = fs::read_dir(path)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .collect();

        let total = entries.len();
        for (i, entry) in entries.into_iter().enumerate() {
            let path = entry.path();
            let progress = (i as f32) / (total as f32);
            progress_sender
                .send((progress, format!("Scanning: {}", path.display())))
                .map_err(|e| e.to_string())?;

            self.process_file(&path)?;
        }

        // Resolve links after scanning
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

        progress_sender
            .send((1.0, "Scan complete".to_string()))
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn process_file(&mut self, path: &Path) -> Result<(), String> {
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                let image_extensions = ["png", "jpg", "jpeg", "gif", "bmp", "webp", "ind"];

                if image_extensions.contains(&ext_lower.as_str()) {
                    self.files.insert(path.to_path_buf(), Vec::new());
                    self.images.push(path.to_path_buf());
                } else if let Ok(content) = fs::read_to_string(path) {
                    let mut links = Vec::new();
                    let link_re = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)|\[\[([^\]]+)\]\]").unwrap();

                    for cap in link_re.captures_iter(&content) {
                        if let Some(link) = cap.get(2) {
                            links.push(PathBuf::from(link.as_str()));
                        } else if let Some(link) = cap.get(3) {
                            links.push(PathBuf::from(link.as_str()));
                        }
                    }

                    self.files.insert(path.to_path_buf(), links);

                    let tag_re = Regex::new(r"#(\w+)").unwrap();
                    let tags: Vec<_> = tag_re
                        .captures_iter(&content)
                        .filter_map(|c| c.get(1))
                        .map(|m| m.as_str().to_string())
                        .collect();
                    if !tags.is_empty() {
                        self.tags.insert(path.to_path_buf(), tags);
                    }
                }
            }
        }
        Ok(())
    }
}
