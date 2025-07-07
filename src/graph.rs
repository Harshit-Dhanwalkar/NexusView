use crate::file_scan;
use petgraph::{Graph, graph::NodeIndex};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct FileGraph {
    pub graph: Graph<String, ()>,
    pub node_indices: HashMap<PathBuf, NodeIndex>,
}

pub struct TagGraph {
    pub graph: Graph<String, ()>,
    pub node_indices: HashMap<PathBuf, NodeIndex>,
}

impl FileGraph {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            node_indices: HashMap::new(),
        }
    }

    pub fn build_from_scanner(&mut self, scanner: &file_scan::FileScanner) {
        for (path, _) in &scanner.files {
            let node_idx = self.graph.add_node(path.display().to_string());
            self.node_indices.insert(path.clone(), node_idx);
        }

        for (source_path, links) in &scanner.files {
            if let Some(&source_idx) = self.node_indices.get(source_path) {
                for target_path in links {
                    if let Some(&target_idx) = self.node_indices.get(target_path) {
                        self.graph.add_edge(source_idx, target_idx, ());
                    }
                }
            }
        }
    }

    pub fn node_indices(&self) -> &HashMap<PathBuf, NodeIndex> {
        &self.node_indices
    }
}

impl TagGraph {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            node_indices: HashMap::new(),
        }
    }

    pub fn build_from_tags(&mut self, scanner: &file_scan::FileScanner) {
        // Add all files with tags as nodes
        for (path, _) in &scanner.files {
            if scanner.tags.contains_key(path) {
                // Only add files that have tags
                let node_idx = self.graph.add_node(path.display().to_string());
                self.node_indices.insert(path.clone(), node_idx);
            }
        }

        // Connect files that share tags
        let file_paths: Vec<PathBuf> = self.node_indices.keys().cloned().collect();
        for i in 0..file_paths.len() {
            for j in (i + 1)..file_paths.len() {
                let path1 = &file_paths[i];
                let path2 = &file_paths[j];

                if let (Some(tags1), Some(tags2)) =
                    (scanner.tags.get(path1), scanner.tags.get(path2))
                {
                    // Check for common tags
                    if tags1.iter().any(|tag1| tags2.contains(tag1)) {
                        if let (Some(&idx1), Some(&idx2)) =
                            (self.node_indices.get(path1), self.node_indices.get(path2))
                        {
                            self.graph.add_edge(idx1, idx2, ());
                        }
                    }
                }
            }
        }
    }
}
