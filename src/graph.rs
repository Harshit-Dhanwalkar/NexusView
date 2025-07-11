// src/graph.rs
use crate::file_scan;
use petgraph::stable_graph::StableGraph;
use petgraph::{Graph, graph::NodeIndex};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GraphNode {
    File(String),
    Tag(String),
}

pub struct FileGraph {
    pub graph: StableGraph<GraphNode, ()>,
    pub node_indices: HashMap<PathBuf, NodeIndex>,
}

pub struct TagGraph {
    pub graph: StableGraph<GraphNode, ()>,
    pub file_node_indices: HashMap<PathBuf, NodeIndex>,
    pub image_node_indices: HashMap<PathBuf, NodeIndex>,
    pub tag_node_indices: HashMap<String, NodeIndex>,
}

impl FileGraph {
    pub fn new() -> Self {
        Self {
            graph: StableGraph::new(),
            node_indices: HashMap::new(),
        }
    }

    pub fn build_from_scanner(&mut self, scanner: &file_scan::FileScanner) {
        self.graph.clear();
        self.node_indices.clear();

        // Add all files as nodes, including orphaned ones
        for (path, _) in &scanner.files {
            let node_data = GraphNode::File(path.display().to_string());
            let node_idx = self.graph.add_node(node_data);
            self.node_indices.insert(path.clone(), node_idx);
        }

        // Add all images as nodes
        for path in &scanner.images {
            if !self.node_indices.contains_key(path) {
                let node_data = GraphNode::File(path.display().to_string());
                let node_idx = self.graph.add_node(node_data);
                self.node_indices.insert(path.clone(), node_idx);
            }
        }

        // Add links between nodes
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
            graph: StableGraph::new(),
            file_node_indices: HashMap::new(),
            tag_node_indices: HashMap::new(),
            image_node_indices: HashMap::new(),
        }
    }

    pub fn build_from_tags(&mut self, scanner: &file_scan::FileScanner) {
        self.graph.clear();
        self.file_node_indices.clear();
        self.image_node_indices.clear();
        self.tag_node_indices.clear();

        // Add all files with tags
        for (file_path, tags) in &scanner.tags {
            if !tags.is_empty() {
                let node_data = GraphNode::File(file_path.display().to_string());
                let node_idx = self.graph.add_node(node_data);
                self.file_node_indices.insert(file_path.clone(), node_idx);
            }
        }

        // Add all images
        for image_path in &scanner.images {
            if !self.image_node_indices.contains_key(image_path) {
                let node_data = GraphNode::File(image_path.display().to_string());
                let node_idx = self.graph.add_node(node_data);
                self.image_node_indices.insert(image_path.clone(), node_idx);
            }
        }

        // Create tag relationships
        for (file_path, tags) in &scanner.tags {
            if let Some(&file_node_idx) = self.file_node_indices.get(file_path) {
                for tag in tags {
                    let tag_node_idx =
                        *self.tag_node_indices.entry(tag.clone()).or_insert_with(|| {
                            let node_data = GraphNode::Tag(tag.clone());
                            self.graph.add_node(node_data)
                        });
                    self.graph.add_edge(tag_node_idx, file_node_idx, ());
                }
            }
        }
    }

    pub fn file_node_indices(&self) -> &HashMap<PathBuf, NodeIndex> {
        &self.file_node_indices
    }

    pub fn tag_node_indices(&self) -> &HashMap<String, NodeIndex> {
        &self.tag_node_indices
    }
}
