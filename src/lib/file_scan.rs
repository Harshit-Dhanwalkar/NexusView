// src/file_scan.rs
use crate::lib::utils::{is_3d_object_path, is_image_path, is_pdf_path};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Error as IoError, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender; // TryRecvError

/// Scans directories and extracts file relationships and tags
pub struct FileScanner {
    root_path: PathBuf,
    pub current_scan_path: PathBuf,
    pub show_hidden: bool,
    pub files: HashMap<PathBuf, Vec<PathBuf>>, // Maps files to their links
    pub images: Vec<PathBuf>,                  // List of image files
    pub objects: Vec<PathBuf>,                 // List of 3D object files
    pub tags: HashMap<PathBuf, Vec<String>>,   // Maps files to their tags
}

impl FileScanner {
    /// Creates a new FileScanner for the given root directory
    pub fn new(root_path: impl AsRef<Path>) -> Self {
        let path = root_path.as_ref().to_path_buf();
        Self {
            root_path: root_path.as_ref().to_path_buf(),
            current_scan_path: path,
            show_hidden: false,
            files: HashMap::new(),
            images: Vec::new(),
            objects: Vec::new(),
            tags: HashMap::new(),
        }
    }

    pub fn set_show_hidden(&mut self, show: bool) {
        self.show_hidden = show;
    }

    pub fn root_path(&self) -> &PathBuf {
        &self.root_path
    }

    /// Scans a directory with progress reporting
    pub fn scan_directory_with_progress(
        &mut self,
        path: &Path,
        progress_sender: Sender<(f32, String)>,
        // ) -> Result<(), String> {
    ) -> Result<(), std::io::Error> {
        if !path.is_dir() {
            return Err(IoError::new(
                ErrorKind::NotFound,
                format!("Path is not a directory: {:?}", path),
            ));
        }

        self.current_scan_path = path.to_path_buf();

        // Clear previous results for this path
        self.files.retain(|k, _| !k.starts_with(path));
        self.tags.retain(|k, _| !k.starts_with(path));
        self.images.retain(|k| !k.starts_with(path));

        // let entries: Vec<_> = fs::read_dir(path)
        //     .map_err(|e| e.to_string())?
        //     .filter_map(|e| e.ok())
        //     .collect();
        let entries: Vec<_> = fs::read_dir(path)?.collect();
        let total_entries = entries.len();
        for (i, entry) in entries.into_iter().enumerate() {
            let entry = entry?;
            let path = entry.path();
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            if file_name.starts_with('.') && !self.show_hidden {
                continue; // Skip hidden files if show_hidden is false
            }

            let progress = (i as f32) / (total_entries as f32);
            let status = format!("Scanning: {}", path.display());
            if let Err(e) = progress_sender.send((progress, status)) {
                return Err(IoError::new(
                    ErrorKind::Other,
                    format!("Failed to send progress update: {}", e),
                ));
            }

            // If a directory, recursively scan it
            if path.is_dir() {
                self.scan_directory_with_progress(&path, progress_sender.clone())?;
            } else if path.is_file() {
                if is_image_path(&path) {
                    self.files.insert(path.clone(), Vec::new());
                    self.images.push(path.clone());
                } else if is_pdf_path(&path) {
                    self.files.insert(path.clone(), Vec::new());
                } else if is_3d_object_path(&path) {
                    self.files.insert(path.clone(), Vec::new());
                    self.objects.push(path.clone());
                } else if let Ok(content) = fs::read_to_string(&path) {
                    self.process_file(&path, &content)?;
                }
            }
        }

        // Resolve links after scanning
        // let mut resolved_files = HashMap::new();
        // for (file_path, links) in &self.files {
        //     let mut resolved_links_for_file = Vec::new();
        //     for link in links {
        //         let resolved_link = if link.is_relative() {
        //             self.current_scan_path.join(link)
        //         } else {
        //             link.clone()
        //         };
        //         resolved_links_for_file.push(resolved_link);
        //     }
        //     resolved_files.insert(file_path.clone(), resolved_links_for_file);
        // }
        // self.files = resolved_files;

        if let Err(e) = progress_sender.send((1.0, "Scan complete".to_string())) {
            return Err(IoError::new(
                ErrorKind::Other,
                format!("Failed to send final progress update: {}", e),
            ));
        }

        Ok(())
    }

    /// Processes an individual file to extract links and tags
    fn process_file(&mut self, path: &Path, _content: &str) -> Result<(), io::Error> {
        if path.is_file() {
            if let Some(_ext) = path.extension().and_then(|e| e.to_str()) {
                if is_image_path(path) {
                    self.files.insert(path.to_path_buf(), Vec::new());
                    self.images.push(path.to_path_buf());
                } else if is_pdf_path(path) {
                    self.files.insert(path.to_path_buf(), Vec::new());
                } else if is_3d_object_path(path) {
                    self.files.insert(path.to_path_buf(), Vec::new());
                    self.objects.push(path.to_path_buf());
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
