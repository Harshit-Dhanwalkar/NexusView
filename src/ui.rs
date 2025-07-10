// src/ui.rs
use eframe::{App, egui};
use egui_commonmark::CommonMarkViewer;
use petgraph::stable_graph::NodeIndex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use crate::file_scan::FileScanner;
use crate::graph::{FileGraph, GraphNode, TagGraph};
use crate::physics_nodes::PhysicsSimulator;
use crate::utils::{is_code_path, is_image_path, is_markdown_path, rotate_vec2};
use egui::{Color32, Sense, Stroke, pos2, vec2};
use once_cell::sync::Lazy;
use petgraph::visit::EdgeRef;
use rand::Rng;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

// Lazy-loaded syntax set and theme
static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| SyntaxSet::load_defaults_newlines());
static THEME_SET: Lazy<ThemeSet> = Lazy::new(|| ThemeSet::load_defaults());
static DEFAULT_THEME: Lazy<&'static Theme> = Lazy::new(|| &THEME_SET.themes["base16-ocean.dark"]);

#[derive(PartialEq)]
enum GraphMode {
    Links,
    Tags,
}

#[derive(Debug, Clone)]
struct DirectoryNode {
    path: PathBuf,
    children: Vec<DirectoryNode>,
    expanded: bool,
    selected: bool,
}

impl DirectoryNode {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            children: Vec::new(),
            expanded: false,
            selected: false,
        }
    }

    fn build_tree(root_path: &Path) -> Self {
        let mut root_node = DirectoryNode::new(root_path.to_path_buf());
        Self::populate_node(&mut root_node);
        root_node
    }

    fn set_selected_recursive(&mut self, target_path: &Path, selected: bool) {
        self.selected = self.path == target_path;
        for child in &mut self.children {
            child.set_selected_recursive(target_path, selected);
        }
    }

    fn update_selection(&mut self, new_selection_path: &Path) {
        self.set_selected_recursive(new_selection_path, false);
        self.set_selected_recursive(new_selection_path, true);
    }

    fn get_selected_directory(&self) -> Option<PathBuf> {
        if self.selected {
            return Some(self.path.clone());
        }
        for child in &self.children {
            if let Some(selected_path) = child.get_selected_directory() {
                return Some(selected_path);
            }
        }
        None
    }

    fn populate_node(node: &mut DirectoryNode) {
        if let Ok(entries) = std::fs::read_dir(&node.path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    let mut child_node = DirectoryNode::new(path);
                    Self::populate_node(&mut child_node);
                    node.children.push(child_node);
                }
            }
        }
    }
}

pub struct FileGraphApp {
    scan_dir: PathBuf,
    show_directory_panel: bool,
    directory_tree: DirectoryNode,
    selected_directory: Option<PathBuf>,
    scanner: Arc<Mutex<FileScanner>>,
    file_graph: FileGraph,
    tag_graph: TagGraph,
    current_graph_mode: GraphMode,
    current_scan_dir: PathBuf,
    show_full_paths: bool,
    physics_simulator: PhysicsSimulator,
    show_physics_menu: bool,
    is_scanning: bool,
    scan_error: Option<String>,
    selected_node: Option<petgraph::graph::NodeIndex>,
    selected_file_content: Option<String>,
    selected_image: Option<egui::TextureHandle>,
    show_content_panel: bool,
    tag_filter_input: String,
    initial_node_layout: HashMap<petgraph::graph::NodeIndex, egui::Vec2>,
    graph_center_offset: egui::Vec2,
    graph_zoom_factor: f32,
    dragged_node: Option<petgraph::graph::NodeIndex>,
    last_drag_pos: Option<egui::Pos2>,
    current_directory_label: String,
    show_images: bool,
    show_hidden_files: bool,
    markdown_cache: egui_commonmark::CommonMarkCache,
    scan_progress: f32,
    scan_status: String,
    graph_rect: egui::Rect,
    graph_build_progress: f32,
    graph_build_status: String,
    scan_sender: Option<std::sync::mpsc::Sender<(f32, String)>>,
    scan_progress_receiver: Option<std::sync::mpsc::Receiver<(f32, String)>>,
    search_query: String,
    search_results: Vec<NodeIndex>,
    current_search_result: usize,
    open_menu_on_node: Option<NodeIndex>,
    right_click_menu_pos: Option<egui::Pos2>,
    menu_open: bool,
    syntax_cache: HashMap<String, SyntaxReference>,
    markdown_syntax: Option<SyntaxReference>,
}

impl App for FileGraphApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Check for scan progress updates
        if let Some(receiver) = self.scan_progress_receiver.take() {
            while let Ok((progress, status)) = receiver.try_recv() {
                self.scan_progress = progress;
                self.scan_status = status;
                if progress >= 1.0 {
                    self.is_scanning = false;
                }
                ctx.request_repaint();
            }
            if self.is_scanning {
                self.scan_progress_receiver = Some(receiver);
            }
        }

        // Update graph building progress
        {
            let scanner_locked = self.scanner.lock().unwrap();

            self.graph_build_progress = 0.0;
            self.graph_build_status = "Building file graph...".to_string();
            ctx.request_repaint();

            // Clear old graphs before rebuilding
            self.file_graph.graph.clear();
            self.file_graph.node_indices.clear();
            self.file_graph.build_from_scanner(&scanner_locked);

            self.graph_build_progress = 0.5;
            self.graph_build_status = "Building tag graph...".to_string();
            ctx.request_repaint();

            // Clear old tag graph before rebuilding
            self.tag_graph.graph.clear();
            self.tag_graph.file_node_indices.clear();
            self.tag_graph.tag_node_indices.clear();
            self.tag_graph.image_node_indices.clear();
            self.tag_graph.build_from_tags(&scanner_locked);

            self.graph_build_progress = 1.0;
            self.graph_build_status = "Graph ready".to_string();
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Scanning directory:");
                ui.monospace(&self.current_directory_label);
            });
            ui.separator();

            ui.horizontal(|ui| {
                // Directory panel toggle button
                if ui.button("📁").clicked() {
                    self.show_directory_panel = !self.show_directory_panel;
                }
                // Content panel toggle button
                if ui.button("📄").clicked() {
                    self.show_content_panel = !self.show_content_panel;
                }
                // Physics menu toggle button
                if ui.button("⚙️").clicked() {
                    self.show_physics_menu = !self.show_physics_menu;
                }
            });

            // Main controls row
            ui.horizontal(|ui| {
                ui.label("Graph Mode:");
                if ui
                    .radio_value(&mut self.current_graph_mode, GraphMode::Links, "Links")
                    .clicked()
                {
                    self.selected_node = None;
                    self.physics_simulator
                        .reset_positions(&self.initial_node_layout);
                }
                if ui
                    .radio_value(&mut self.current_graph_mode, GraphMode::Tags, "Tags")
                    .clicked()
                {
                    self.selected_node = None;
                    self.physics_simulator
                        .reset_positions(&self.initial_node_layout);
                }

                ui.checkbox(&mut self.show_full_paths, "Show Full Paths");
                ui.checkbox(&mut self.show_images, "Show Images");

                if ui
                    .checkbox(&mut self.show_hidden_files, "Show Hidden Files")
                    .changed()
                {
                    if let Ok(mut scanner_guard) = self.scanner.lock() {
                        scanner_guard.set_show_hidden(self.show_hidden_files);
                    } else {
                        eprintln!("Failed to lock scanner mutex when setting show_hidden.");
                        return;
                    }

                    // Trigger a rescan of the currently selected directory
                    if !self.is_scanning {
                        let scan_dir = self
                            .selected_directory
                            .clone()
                            .unwrap_or_else(|| self.scan_dir.clone());
                        self.current_scan_dir = scan_dir.clone();
                        self.trigger_scan(scan_dir, ctx);
                    }
                }

                ui.separator();

                ui.label("Filter Tags:");
                ui.text_edit_singleline(&mut self.tag_filter_input);

                if ui.button("Rescan Directory").clicked() && !self.is_scanning {
                    self.scan_error = None;
                    self.is_scanning = true;
                    self.scan_progress = 0.0;
                    self.scan_status = "Starting scan...".to_string();

                    let scan_dir = self
                        .selected_directory
                        .clone()
                        .unwrap_or_else(|| self.scan_dir.clone());
                    self.current_scan_dir = scan_dir.clone();

                    let scanner_arc_clone = self.scanner.clone();
                    let (progress_sender, progress_receiver) = std::sync::mpsc::channel();

                    thread::spawn(move || {
                        let mut scanner = scanner_arc_clone.lock().unwrap();
                        if let Err(e) =
                            scanner.scan_directory_with_progress(&scan_dir, progress_sender)
                        {
                            eprintln!("Error during scan: {}", e);
                        }
                    });

                    self.scan_progress_receiver = Some(progress_receiver);
                }

                if self.is_scanning {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.add(egui::ProgressBar::new(self.scan_progress).show_percentage());
                        ui.label(&self.scan_status);
                    });
                }

                if let Some(ref err) = self.scan_error {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
                }

                if ui
                    .add(egui::Button::new("✕ Exit").fill(Color32::from_rgb(200, 80, 80)))
                    .clicked()
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            // Physics controls section
            if self.show_physics_menu {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.group(|ui| {
                        ui.label("Physics Controls:");
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.physics_simulator.spring_constant,
                                        0.001..=0.5,
                                    )
                                    .text("Spring K"),
                                );
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.physics_simulator.damping,
                                        0.0..=0.9,
                                    )
                                    .text("Damping"),
                                );
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.physics_simulator.time_step,
                                        0.1..=1.0,
                                    )
                                    .text("Time Step"),
                                );
                            });
                            ui.vertical(|ui| {
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.physics_simulator.repulsion_constant,
                                        100.0..=50000.0,
                                    )
                                    .text("Repulsion K"),
                                );
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.physics_simulator.ideal_edge_length,
                                        10.0..=300.0,
                                    )
                                    .text("Ideal Length"),
                                );
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.physics_simulator.friction,
                                        0.0..=0.9,
                                    )
                                    .text("Friction"),
                                );
                            });
                        });

                        if ui.button("Reset Node Positions").clicked() {
                            self.physics_simulator
                                .reset_positions(&self.initial_node_layout);
                        }

                        if ui.button("Center Graph").clicked() {
                            self.center_graph();
                        }
                    });
                });
            }

            // Graph Search section
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Search:");
                if ui.text_edit_singleline(&mut self.search_query).changed() {
                    self.perform_search();
                }

                if !self.search_results.is_empty() {
                    ui.label(format!(
                        "{} of {}",
                        self.current_search_result + 1,
                        self.search_results.len()
                    ));
                    if ui.button("◀").clicked() {
                        self.focus_prev_search_result();
                    }
                    if ui.button("▶").clicked() {
                        self.focus_next_search_result();
                    }
                }
            });
        });

        // Left directory panel
        let panel_width = 200.0;
        let panel_response = egui::SidePanel::left("directory_panel")
            .resizable(true)
            .default_width(panel_width)
            .show_animated(ctx, self.show_directory_panel, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Directories");
                    if ui.button("◀").clicked() {
                        self.show_directory_panel = !self.show_directory_panel;
                    }
                });

                // Add scan selected button
                if ui.button("📂 Scan Selected").clicked() {
                    self.scan_selected_directories(ctx);
                }

                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let tree = &mut self.directory_tree;
                    FileGraphApp::render_directory_tree_node(ui, tree);
                });
            });

        // Central panel
        let central_response =
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| {
                    let available_width = if self.show_directory_panel {
                        ui.available_width()
                            - panel_response
                                .as_ref()
                                .map_or(0.0, |r| r.response.rect.width())
                    } else {
                        ui.available_width()
                    };
                    ui.set_width(available_width);

                    let (response, painter) = ui.allocate_painter(
                        ui.available_size(),
                        egui::Sense::hover() | egui::Sense::drag() | egui::Sense::click(),
                    );

                    self.graph_rect = response.rect;
                    let graph_rect = response.rect;
                    let to_screen = egui::emath::RectTransform::from_to(
                        egui::Rect::from_center_size(egui::Pos2::ZERO, graph_rect.size()),
                        graph_rect,
                    );

                    if self.graph_build_progress < 1.0 {
                        ui.with_layout(
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                            |ui| {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.add(
                                        egui::ProgressBar::new(self.graph_build_progress)
                                            .show_percentage(),
                                    );
                                    ui.label(&self.graph_build_status);
                                });
                            },
                        );
                    }

                    if response.hovered() {
                        self.graph_zoom_factor *= ctx.input(|i| i.zoom_delta());
                        self.graph_zoom_factor = self.graph_zoom_factor.clamp(0.1, 10.0);
                    }

                    if response.dragged_by(egui::PointerButton::Middle) {
                        self.graph_center_offset += response.drag_delta() / self.graph_zoom_factor;
                    }

                    if ctx.input(|i| i.key_pressed(egui::Key::F)) {
                        if let Some(node_idx) = self.selected_node {
                            self.focus_on_node(node_idx);
                        }
                    }

                    if ctx.input(|i| i.key_pressed(egui::Key::F3)) {
                        self.focus_next_search_result();
                    }

                    if ctx.input(|i| i.key_pressed(egui::Key::F3) && i.modifiers.shift) {
                        self.focus_prev_search_result();
                    }

                    {
                        let scanner_locked = self.scanner.lock().unwrap();
                        self.file_graph.build_from_scanner(&scanner_locked);
                        self.tag_graph.build_from_tags(&scanner_locked);
                    }

                    // node filtering logic:
                    let (nodes_to_draw, edges_to_draw) = {
                        let scanner_locked = self.scanner.lock().unwrap();

                        match self.current_graph_mode {
                            GraphMode::Links => {
                                let mut nodes = Vec::new();
                                let mut edges = Vec::new();

                                // Add all files
                                for (path, node_idx) in &self.file_graph.node_indices {
                                    let is_image = is_image_path(path);

                                    if self.show_images || !is_image {
                                        nodes.push(*node_idx);
                                    }
                                }

                                // Add all edges between visible nodes
                                for edge in self.file_graph.graph.edge_references() {
                                    if nodes.contains(&edge.source())
                                        && nodes.contains(&edge.target())
                                    {
                                        edges.push((edge.source(), edge.target()));
                                    }
                                }

                                (nodes, edges)
                            }
                            GraphMode::Tags => {
                                let filtered_tag_nodes: HashMap<_, _> = self
                                    .tag_graph
                                    .tag_node_indices
                                    .iter()
                                    .filter(|(tag_name, _)| {
                                        self.tag_filter_input.is_empty()
                                            || tag_name.contains(&self.tag_filter_input)
                                    })
                                    .map(|(tag_name, &node_idx)| (tag_name.clone(), node_idx))
                                    .collect();

                                let mut nodes = Vec::new();
                                let mut edges = Vec::new();

                                // Always include all file nodes with tags
                                nodes.extend(self.tag_graph.file_node_indices.values());

                                // Include images if show_images is true
                                if self.show_images {
                                    nodes.extend(self.tag_graph.image_node_indices.values());
                                }

                                // Include tag nodes that match the filter
                                for (_, &tag_node_idx) in &filtered_tag_nodes {
                                    nodes.push(tag_node_idx);
                                    for edge_ref in self.tag_graph.graph.edges(tag_node_idx) {
                                        edges.push((edge_ref.source(), edge_ref.target()));
                                    }
                                }
                                (nodes, edges)
                            }
                        }
                    };

                    // Clear any old nodes from physics simulator that aren't in current graph
                    self.physics_simulator
                        .node_positions
                        .retain(|node_idx, _| nodes_to_draw.contains(node_idx));
                    self.physics_simulator
                        .node_velocities
                        .retain(|node_idx, _| nodes_to_draw.contains(node_idx));
                    self.initial_node_layout
                        .retain(|node_idx, _| nodes_to_draw.contains(node_idx));

                    for node_idx in &nodes_to_draw {
                        if !self.physics_simulator.node_positions.contains_key(node_idx) {
                            let mut rng = rand::rng();
                            let random_pos = egui::vec2(
                                rng.random_range(-100.0..100.0),
                                rng.random_range(-100.0..100.0),
                            );
                            self.physics_simulator
                                .node_positions
                                .insert(*node_idx, random_pos);
                            self.physics_simulator
                                .node_velocities
                                .insert(*node_idx, egui::Vec2::ZERO);
                            self.initial_node_layout.insert(*node_idx, random_pos);
                        }
                    }

                    self.physics_simulator
                        .node_positions
                        .retain(|node_idx, _| nodes_to_draw.contains(node_idx));
                    self.physics_simulator
                        .node_velocities
                        .retain(|node_idx, _| nodes_to_draw.contains(node_idx));
                    self.initial_node_layout
                        .retain(|node_idx, _| nodes_to_draw.contains(node_idx));

                    if self.dragged_node.is_none() {
                        self.physics_simulator.update(&edges_to_draw);
                    } else {
                        let original_time_step = self.physics_simulator.time_step;
                        self.physics_simulator.time_step = original_time_step * 0.4;
                        self.physics_simulator.update(&edges_to_draw);
                        self.physics_simulator.time_step = original_time_step;
                    }

                    // Animation effects
                    let time = ctx.input(|i| i.time) as f32;
                    let global_pulse = (time * 2.0).sin() * 0.02 + 1.0;

                    // Draw edges with enhanced styling
                    for (start_node_idx, end_node_idx) in &edges_to_draw {
                        if let (Some(&start_pos), Some(&end_pos)) = (
                            self.physics_simulator.get_node_position(*start_node_idx),
                            self.physics_simulator.get_node_position(*end_node_idx),
                        ) {
                            let start_screen_pos = to_screen.transform_pos(pos2(
                                start_pos.x * self.graph_zoom_factor + self.graph_center_offset.x,
                                start_pos.y * self.graph_zoom_factor + self.graph_center_offset.y,
                            ));
                            let end_screen_pos = to_screen.transform_pos(pos2(
                                end_pos.x * self.graph_zoom_factor + self.graph_center_offset.x,
                                end_pos.y * self.graph_zoom_factor + self.graph_center_offset.y,
                            ));

                            let vec_between = end_screen_pos - start_screen_pos;
                            let dir = vec_between.normalized();

                            // Enhanced edge drawing with glow effect
                            let edge_stroke = Stroke::new(
                                1.5 * self.graph_zoom_factor,
                                Color32::from_rgba_premultiplied(100, 100, 255, 150),
                            );

                            // Draw the edge with glow effect
                            for i in 0..3 {
                                let width = edge_stroke.width - i as f32 * 0.5;
                                let alpha = (150 - i * 50) as f32;
                                let glow_stroke = Stroke::new(
                                    width,
                                    Color32::from_rgba_premultiplied(100, 100, 255, alpha as u8),
                                );
                                painter
                                    .line_segment([start_screen_pos, end_screen_pos], glow_stroke);
                            }

                            // Draw the main edge
                            painter.line_segment([start_screen_pos, end_screen_pos], edge_stroke);

                            // Arrow with glow
                            let arrow_size = 10.0 * self.graph_zoom_factor;
                            let arrow_tip1 = end_screen_pos - rotate_vec2(dir, 0.5) * arrow_size;
                            let arrow_tip2 = end_screen_pos - rotate_vec2(dir, -0.5) * arrow_size;

                            for i in 0..3 {
                                let width = edge_stroke.width - i as f32 * 0.5;
                                let alpha = (150 - i * 50) as f32;
                                let glow_stroke = Stroke::new(
                                    width,
                                    Color32::from_rgba_premultiplied(100, 100, 255, alpha as u8),
                                );
                                painter.line_segment([end_screen_pos, arrow_tip1], glow_stroke);
                                painter.line_segment([end_screen_pos, arrow_tip2], glow_stroke);
                            }

                            painter.line_segment([end_screen_pos, arrow_tip1], edge_stroke);
                            painter.line_segment([end_screen_pos, arrow_tip2], edge_stroke);
                        }
                    }

                    // Draw nodes with enhanced styling
                    for &node_idx in &nodes_to_draw {
                        if let Some(node_pos_vec2) =
                            self.physics_simulator.get_node_position(node_idx).cloned()
                        {
                            let screen_pos = to_screen.transform_pos(pos2(
                                node_pos_vec2.x * self.graph_zoom_factor
                                    + self.graph_center_offset.x,
                                node_pos_vec2.y * self.graph_zoom_factor
                                    + self.graph_center_offset.y,
                            ));

                            let node_name = match self.current_graph_mode {
                                GraphMode::Links => match &self.file_graph.graph[node_idx] {
                                    GraphNode::File(s) => s.clone(),
                                    GraphNode::Tag(s) => s.clone(),
                                },
                                GraphMode::Tags => match &self.tag_graph.graph[node_idx] {
                                    GraphNode::File(s) => s.clone(),
                                    GraphNode::Tag(s) => s.clone(),
                                },
                            };

                            // Enhanced node styling parameters
                            let node_radius = 15.0 * self.graph_zoom_factor * global_pulse;
                            let node_color = if Some(node_idx) == self.selected_node {
                                Color32::from_rgb(255, 100, 100)
                            } else if self.search_results.contains(&node_idx) {
                                Color32::from_rgb(100, 255, 100)
                            } else {
                                match self.current_graph_mode {
                                    GraphMode::Links => match &self.file_graph.graph[node_idx] {
                                        GraphNode::File(path) => {
                                            let path = Path::new(path);
                                            let is_image = is_image_path(path);
                                            if is_image {
                                                Color32::from_rgb(255, 165, 0) // Orange for images
                                            } else if is_markdown_path(path) {
                                                Color32::from_rgb(100, 200, 255) // Blue for markdown
                                            } else if is_code_path(path) {
                                                Color32::from_rgb(150, 100, 255) // Purple for code
                                            } else {
                                                Color32::from_rgb(100, 200, 150) // Teal for other files
                                            }
                                        }
                                        GraphNode::Tag(_) => Color32::from_rgb(255, 100, 150), // Pink for tags
                                    },
                                    GraphMode::Tags => match &self.tag_graph.graph[node_idx] {
                                        GraphNode::File(path) => {
                                            let scanner_locked = self.scanner.lock().unwrap();
                                            let has_tags =
                                                scanner_locked.tags.contains_key(Path::new(path));
                                            let is_image = is_image_path(Path::new(path));
                                            if is_image {
                                                Color32::from_rgb(255, 165, 0) // Orange for images
                                            } else if has_tags {
                                                Color32::from_rgb(100, 200, 255) // Blue for tagged files
                                            } else {
                                                Color32::from_rgb(100, 100, 100) // Gray for untagged files
                                            }
                                        }
                                        GraphNode::Tag(_) => Color32::from_rgb(255, 100, 150), // Pink for tags
                                    },
                                }
                            };

                            // Custom node styling parameters
                            let node_glow_radius = 10.0 * self.graph_zoom_factor;
                            let node_shadow_offset = vec2(2.0, 2.0) * self.graph_zoom_factor;

                            // Pulse effect for selected node
                            let pulse = if Some(node_idx) == self.selected_node {
                                (time as f32).sin().abs() * 0.2 + 0.8
                            } else {
                                1.0
                            };

                            // Draw the node with effects
                            if Some(node_idx) == self.selected_node {
                                // Glow effect for selected node
                                for i in 0..5 {
                                    let radius = node_radius * pulse + i as f32 * 2.0;
                                    let alpha = (50 - i * 10) as f32 / 255.0;
                                    let glow_color = Color32::from_rgba_premultiplied(
                                        node_color.r(),
                                        node_color.g(),
                                        node_color.b(),
                                        (alpha * 255.0) as u8,
                                    );
                                    painter.circle_stroke(
                                        screen_pos,
                                        radius,
                                        Stroke::new(2.0, glow_color),
                                    );
                                }
                            }

                            // Node shadow
                            painter.circle_filled(
                                screen_pos + node_shadow_offset,
                                node_radius,
                                Color32::from_black_alpha(50),
                            );

                            // Main node circle
                            painter.circle_filled(screen_pos, node_radius, node_color);

                            // Node border
                            let border_color = if Some(node_idx) == self.selected_node {
                                Color32::WHITE
                            } else {
                                Color32::from_gray(100)
                            };
                            painter.circle_stroke(
                                screen_pos,
                                node_radius,
                                Stroke::new(1.5, border_color),
                            );

                            // Node label with improved styling
                            let display_name = if self.show_full_paths {
                                node_name.clone()
                            } else {
                                PathBuf::from(&node_name)
                                    .file_name()
                                    .and_then(|os_str| os_str.to_str())
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| node_name.clone())
                            };

                            let font_id = egui::TextStyle::Body.resolve(ui.style());
                            let text_color = {
                                let r = node_color.r() as f32 / 255.0;
                                let g = node_color.g() as f32 / 255.0;
                                let b = node_color.b() as f32 / 255.0;
                                let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                                if luminance > 0.5 {
                                    Color32::BLACK
                                } else {
                                    Color32::WHITE
                                }
                            };

                            let text_galley =
                                ui.fonts(|f| f.layout_no_wrap(display_name, font_id, text_color));

                            // Text background for better readability
                            let text_bg_rect = egui::Rect::from_center_size(
                                screen_pos
                                    + vec2(0.0, node_radius + 5.0 + text_galley.size().y / 2.0),
                                text_galley.size() + vec2(8.0, 4.0),
                            );
                            painter.rect_filled(text_bg_rect, 4.0, Color32::from_black_alpha(200));
                            painter.galley(
                                text_bg_rect.center() - text_galley.size() / 2.0,
                                text_galley.clone(),
                                text_color,
                            );

                            let node_rect = if text_galley.size().y > 0.0 {
                                egui::Rect::from_center_size(
                                    screen_pos,
                                    egui::vec2(
                                        node_radius * 2.0,
                                        node_radius * 2.0 + text_galley.size().y,
                                    ),
                                )
                            } else {
                                egui::Rect::from_center_size(
                                    screen_pos,
                                    egui::vec2(node_radius * 2.0, node_radius * 2.0),
                                )
                            };

                            let node_response = ui.interact(
                                node_rect,
                                ui.id().with(node_idx),
                                Sense::click_and_drag(),
                            );

                            if node_response.dragged_by(egui::PointerButton::Primary) {
                                let delta = node_response.drag_delta() / self.graph_zoom_factor;
                                self.physics_simulator
                                    .set_node_position(node_idx, node_pos_vec2 + delta);
                                self.dragged_node = Some(node_idx);
                                self.last_drag_pos = Some(node_response.rect.center());
                            } else if node_response.drag_stopped() {
                                self.dragged_node = None;
                                self.last_drag_pos = None;
                            }

                            // Enhanced hover effects
                            if node_response.hovered() {
                                // Glow effect on hover
                                for i in 0..3 {
                                    let radius = node_radius + i as f32 * 3.0;
                                    let alpha = (100 - i * 30) as f32;
                                    let hover_color = Color32::from_rgba_premultiplied(
                                        node_color.r(),
                                        node_color.g(),
                                        node_color.b(),
                                        alpha as u8,
                                    );
                                    painter.circle_stroke(
                                        screen_pos,
                                        radius,
                                        Stroke::new(2.0, hover_color),
                                    );
                                }

                                // Show tooltip with additional information
                                let full_name = match self.current_graph_mode {
                                    GraphMode::Links => match &self.file_graph.graph[node_idx] {
                                        GraphNode::File(file_path_str) => file_path_str.clone(),
                                        GraphNode::Tag(tag_name) => format!("#{}", tag_name),
                                    },
                                    GraphMode::Tags => match &self.tag_graph.graph[node_idx] {
                                        GraphNode::File(file_path_str) => file_path_str.clone(),
                                        GraphNode::Tag(tag_name) => format!("#{}", tag_name),
                                    },
                                };

                                let tooltip_content = match self.current_graph_mode {
                                    GraphMode::Links => {
                                        if let GraphNode::File(path) =
                                            &self.file_graph.graph[node_idx]
                                        {
                                            let file_type = if is_image_path(Path::new(path)) {
                                                "Image"
                                            } else if is_markdown_path(Path::new(path)) {
                                                "Markdown"
                                            } else if is_code_path(Path::new(path)) {
                                                "Code"
                                            } else {
                                                "File"
                                            };
                                            format!("{}: {}", file_type, full_name)
                                        } else {
                                            full_name
                                        }
                                    }
                                    GraphMode::Tags => full_name,
                                };

                                egui::show_tooltip_at(
                                    ctx,
                                    egui::LayerId::new(
                                        egui::Order::Tooltip,
                                        egui::Id::new("node_tooltip"),
                                    ),
                                    egui::Id::new("node_tooltip"),
                                    node_response.hover_pos().unwrap(),
                                    |ui| {
                                        ui.label(egui::RichText::new(tooltip_content).strong());
                                        if let GraphNode::File(path) =
                                            &self.file_graph.graph[node_idx]
                                        {
                                            if let Ok(metadata) = std::fs::metadata(path) {
                                                let modified =
                                                    metadata.modified().unwrap_or_else(|_| {
                                                        std::time::SystemTime::UNIX_EPOCH
                                                    });
                                                let size = metadata.len();
                                                ui.label(format!("Size: {} bytes", size));
                                                ui.label(format!("Modified: {:?}", modified));
                                            }
                                        }
                                    },
                                );
                            }

                            if node_response.clicked_by(egui::PointerButton::Primary) {
                                self.selected_node = Some(node_idx);
                                self.selected_file_content = None; // Clear previous content
                                self.selected_image = None; // Clear previous image

                                match self.current_graph_mode {
                                    GraphMode::Links => {
                                        if let GraphNode::File(file_path_str) =
                                            &self.file_graph.graph[node_idx]
                                        {
                                            self.try_load_file_content(file_path_str.into(), ctx);
                                        }
                                    }
                                    GraphMode::Tags => {
                                        if let GraphNode::File(file_path_str) =
                                            &self.tag_graph.graph[node_idx]
                                        {
                                            self.try_load_file_content(file_path_str.into(), ctx);
                                        }
                                    }
                                }
                                self.show_content_panel = true; // Show content panel on node click
                            }

                            if node_response.clicked_by(egui::PointerButton::Secondary) {
                                self.open_menu_on_node = Some(node_idx);
                                self.right_click_menu_pos = node_response.hover_pos();
                                self.menu_open = true;
                            }
                        }
                    }

                    // Render the custom right-click menu as an egui::Window
                    if let Some(menu_node_idx) = self.open_menu_on_node {
                        if let Some(menu_pos) = self.right_click_menu_pos {
                            // Use the stored mouse position
                            let mut should_close_menu = false;

                            let window_response = egui::Window::new("Node Actions")
                                .id(egui::Id::new("right_click_node_menu").with(menu_node_idx))
                                .default_pos(menu_pos)
                                .collapsible(false)
                                .resizable(false)
                                .default_width(200.0)
                                .show(ctx, |ui| {
                                    let full_name_for_menu = match self.current_graph_mode {
                                        GraphMode::Links => match &self.file_graph.graph
                                            [menu_node_idx]
                                        {
                                            GraphNode::File(file_path_str) => file_path_str.clone(),
                                            GraphNode::Tag(tag_name) => {
                                                format!("Tag: #{}", tag_name)
                                            }
                                        },
                                        GraphMode::Tags => match &self.tag_graph.graph
                                            [menu_node_idx]
                                        {
                                            GraphNode::File(file_path_str) => file_path_str.clone(),
                                            GraphNode::Tag(tag_name) => {
                                                format!("Tag: #{}", tag_name)
                                            }
                                        },
                                    };
                                    ui.label(full_name_for_menu);
                                    ui.separator();

                                    let path_buf_option = match self.current_graph_mode {
                                        GraphMode::Links => {
                                            match &self.file_graph.graph[menu_node_idx] {
                                                GraphNode::File(s) => Some(PathBuf::from(s)),
                                                GraphNode::Tag(_) => None,
                                            }
                                        }
                                        GraphMode::Tags => {
                                            match &self.tag_graph.graph[menu_node_idx] {
                                                GraphNode::File(s) => Some(PathBuf::from(s)),
                                                GraphNode::Tag(_) => None,
                                            }
                                        }
                                    };

                                    if let Some(path_buf) = path_buf_option {
                                        if path_buf.is_file() {
                                            if ui.button("Open File").clicked() {
                                                #[cfg(target_os = "linux")]
                                                {
                                                    std::process::Command::new("xdg-open")
                                                        .arg(&path_buf)
                                                        .spawn()
                                                        .expect("Failed to open file");
                                                }
                                                #[cfg(target_os = "macos")]
                                                {
                                                    std::process::Command::new("open")
                                                        .arg(&path_buf)
                                                        .spawn()
                                                        .expect("Failed to open file");
                                                }
                                                #[cfg(target_os = "windows")]
                                                {
                                                    std::process::Command::new("cmd")
                                                        .arg("/C")
                                                        .arg("start")
                                                        .arg(&path_buf)
                                                        .spawn()
                                                        .expect("Failed to open file");
                                                }
                                                should_close_menu = true;
                                            }
                                            if ui.button("Copy Path").clicked() {
                                                ctx.copy_text(
                                                    path_buf.to_string_lossy().to_string(),
                                                );
                                                should_close_menu = true;
                                            }
                                        }
                                    }
                                });

                            // Check the window's response to see if it was closed
                            if window_response.is_none()
                                || (window_response.is_some()
                                    && window_response.unwrap().response.clicked_elsewhere())
                                || should_close_menu
                            {
                                self.open_menu_on_node = None;
                                self.right_click_menu_pos = None;
                                self.menu_open = false;
                            } else {
                                self.menu_open = true;
                            }
                        }
                    } else {
                        self.menu_open = false;
                    }
                });

        // Right panel for file content
        egui::SidePanel::right("file_content_panel")
            .min_width(200.0)
            .show_animated(ctx, self.show_content_panel, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("File Content");
                    if ui.button("▶").clicked() {
                        self.show_content_panel = !self.show_content_panel;
                    }
                });
                ui.separator();

                // Display file name
                if let Some(node_idx) = self.selected_node {
                    let file_name = match self.current_graph_mode {
                        GraphMode::Links => match &self.file_graph.graph[node_idx] {
                            GraphNode::File(s) => PathBuf::from(s).file_name().map_or_else(
                                || s.clone(),
                                |os_str| os_str.to_string_lossy().into_owned(),
                            ),
                            GraphNode::Tag(s) => format!("#{}", s),
                        },
                        GraphMode::Tags => match &self.tag_graph.graph[node_idx] {
                            GraphNode::File(s) => PathBuf::from(s).file_name().map_or_else(
                                || s.clone(),
                                |os_str| os_str.to_string_lossy().into_owned(),
                            ),
                            GraphNode::Tag(s) => format!("#{}", s),
                        },
                    };

                    ui.label(egui::RichText::new(file_name).strong());
                    ui.separator();
                }

                if let Some(content) = &self.selected_file_content {
                    if self.is_markdown_file() {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            CommonMarkViewer::new().show(ui, &mut self.markdown_cache, content);
                        });
                    } else if self.is_code_file() {
                        let content_clone = content.clone();
                        self.render_code_with_syntax_highlighting(ui, &content_clone);
                    } else {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.label(content);
                        });
                    }
                } else if let Some(image) = &self.selected_image {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // Show image dimensions
                        let size = image.size_vec2();
                        ui.label(format!("Dimensions: {} × {} px", size.x, size.y));
                        ui.add_space(10.0);
                        ui.image(image);
                    });
                } else {
                    ui.label("Select a file node to view its content.");
                }
            });
    }
}

impl FileGraphApp {
    pub fn new(scan_dir: PathBuf) -> Self {
        let scanner = Arc::new(Mutex::new(FileScanner::new(&scan_dir)));
        let directory_tree = DirectoryNode::build_tree(&scan_dir);
        let (progress_sender, progress_receiver) = std::sync::mpsc::channel();

        let mut app = Self {
            scan_dir: scan_dir.clone(),
            show_directory_panel: true,
            directory_tree,
            selected_directory: None,
            current_scan_dir: scan_dir.clone(),
            scanner: scanner.clone(),
            file_graph: FileGraph::new(),
            tag_graph: TagGraph::new(),
            current_graph_mode: GraphMode::Links,
            show_full_paths: false,
            physics_simulator: PhysicsSimulator::new(),
            show_physics_menu: true,
            is_scanning: false,
            scan_error: None,
            selected_node: None,
            selected_file_content: None,
            selected_image: None,
            tag_filter_input: String::new(),
            initial_node_layout: HashMap::new(),
            graph_center_offset: egui::Vec2::ZERO,
            graph_zoom_factor: 1.0,
            dragged_node: None,
            last_drag_pos: None,
            current_directory_label: scan_dir.display().to_string(),
            show_images: true,
            // show_orphans: true,
            show_hidden_files: false,
            graph_rect: egui::Rect::NOTHING,
            markdown_cache: egui_commonmark::CommonMarkCache::default(),
            scan_progress: 0.0,
            scan_status: String::new(),
            graph_build_progress: 0.0,
            graph_build_status: "Ready".to_string(),
            scan_sender: Some(progress_sender),
            scan_progress_receiver: Some(progress_receiver),
            search_query: String::new(),
            search_results: Vec::new(),
            current_search_result: 0,
            open_menu_on_node: None,
            right_click_menu_pos: None,
            menu_open: false,
            syntax_cache: HashMap::new(),
            markdown_syntax: SYNTAX_SET.find_syntax_by_extension("md").cloned(),
            show_content_panel: true,
        };

        if let Some(initial_scan_path) = app.selected_directory.clone() {
            app.trigger_scan(initial_scan_path.clone(), &egui::Context::default());
        }

        app
    }

    fn trigger_scan(&mut self, path_to_scan: PathBuf, ctx: &egui::Context) {
        // Early return checks
        if self.is_scanning {
            eprintln!("Already scanning, ignoring new scan request.");
            return;
        }

        // Validate path
        if !path_to_scan.is_dir() {
            self.scan_error = Some("Selected path is not a directory".to_string());
            return;
        }

        // Update UI state
        self.is_scanning = true;
        self.scan_progress = 0.0;
        self.scan_status = format!("Scanning: {}", path_to_scan.display());
        self.current_scan_dir = path_to_scan.clone();
        self.scan_error = None;
        self.current_directory_label = path_to_scan.display().to_string();

        // Clear old data
        self.clear_graph_data();

        // Prepare for new scan
        let scanner_arc_clone = self.scanner.clone();
        let (progress_sender, progress_receiver) = std::sync::mpsc::channel();
        self.scan_sender = Some(progress_sender.clone());
        self.scan_progress_receiver = Some(progress_receiver);

        let ctx_clone = ctx.clone();
        let show_hidden_clone = self.show_hidden_files;

        // Spawn scanning thread
        thread::spawn(move || {
            let scan_start_time = Instant::now();

            match scanner_arc_clone.lock() {
                Ok(mut scanner_guard) => {
                    scanner_guard.set_show_hidden(show_hidden_clone);

                    match scanner_guard.scan_directory_with_progress(&path_to_scan, progress_sender)
                    {
                        Ok(_) => {
                            println!("Scan completed in {:?}", scan_start_time.elapsed());
                        }
                        Err(e) => {
                            eprintln!("Scan error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to lock scanner: {}", e);
                }
            }

            ctx_clone.request_repaint();
        });
    }

    // Helper function to clear graph data
    fn clear_graph_data(&mut self) {
        // Clear physics data
        self.physics_simulator.node_positions.clear();
        self.physics_simulator.node_velocities.clear();
        self.initial_node_layout.clear();

        // Clear graph structures
        self.file_graph.graph.clear();
        self.file_graph.node_indices.clear();
        self.tag_graph.graph.clear();
        self.tag_graph.file_node_indices.clear();
        self.tag_graph.tag_node_indices.clear();
        self.tag_graph.image_node_indices.clear();

        // Clear UI state
        self.selected_node = None;
        self.selected_file_content = None;
        self.selected_image = None;
        self.search_results.clear();
        self.current_search_result = 0;
    }

    fn build_graphs(&mut self) {
        self.graph_build_progress = 0.0;
        self.graph_build_status = "Building graphs...".to_string();

        let scanner_guard = self
            .scanner
            .lock()
            .expect("Failed to lock scanner for graph build");
        self.file_graph.build_from_scanner(&scanner_guard);
        self.tag_graph.build_from_tags(&scanner_guard);
        drop(scanner_guard); // Release lock

        // Calculate initial layout for physics simulation
        self.initial_node_layout.clear();
        let mut rng = rand::rng();
        let graph_center = self.graph_rect.center();
        let radius = self.graph_rect.width().min(self.graph_rect.height()) / 3.0;

        // Use the combined nodes from both graphs to initialize physics
        let mut all_node_indices: HashMap<NodeIndex, GraphNode> = HashMap::new();
        for (idx, node) in self.file_graph.graph.node_weights().enumerate() {
            all_node_indices.insert(NodeIndex::new(idx), node.clone());
        }
        for (idx, node) in self.tag_graph.graph.node_weights().enumerate() {
            all_node_indices.insert(NodeIndex::new(idx), node.clone());
        }

        for (node_idx, _) in &all_node_indices {
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let x = graph_center.x + radius * angle.cos();
            let y = graph_center.y + radius * angle.sin();
            self.initial_node_layout.insert(*node_idx, egui::vec2(x, y));
        }

        self.physics_simulator.node_positions = self.initial_node_layout.clone();
        self.physics_simulator.initialize_velocities();

        self.graph_build_progress = 1.0;
        self.graph_build_status = "Graphs built.".to_string();
    }

    fn draw_directory_node_recursive(
        &mut self,
        ui: &mut egui::Ui,
        node: &mut DirectoryNode,
        ctx: &egui::Context,
    ) {
        ui.collapsing(
            node.path.file_name().unwrap_or_default().to_string_lossy(),
            |ui| {
                let response = ui.selectable_label(node.selected, node.path.display().to_string());

                if response.clicked() {
                    self.directory_tree.update_selection(&node.path);
                    self.selected_directory = Some(node.path.clone());
                    self.current_scan_dir = node.path.clone();
                    self.current_directory_label = node.path.display().to_string();

                    // When a directory is selected, trigger a scan for that directory
                    self.trigger_scan(node.path.clone(), ctx);
                }
            },
        );
    }

    fn scan_selected_directories(&mut self, ctx: &egui::Context) {
        let mut selected_paths = Vec::new();
        self.collect_selected_paths(&self.directory_tree, &mut selected_paths);

        if !selected_paths.is_empty() {
            self.scan_error = None;
            self.is_scanning = true;
            self.scan_progress = 0.0;
            self.scan_status = "Starting scan...".to_string();

            // Clear old physics data
            self.physics_simulator.node_positions.clear();
            self.physics_simulator.node_velocities.clear();
            self.initial_node_layout.clear();

            let scanner_arc_clone = self.scanner.clone();
            let (progress_sender, progress_receiver) = std::sync::mpsc::channel();

            thread::spawn(move || {
                let mut scanner = scanner_arc_clone.lock().unwrap();
                // Clear previous results before scanning new directories
                scanner.files.clear();
                scanner.tags.clear();
                scanner.images.clear();

                for path in selected_paths {
                    if let Err(e) =
                        scanner.scan_directory_with_progress(&path, progress_sender.clone())
                    {
                        eprintln!("Error scanning {}: {}", path.display(), e);
                    }
                }
            });

            self.scan_progress_receiver = Some(progress_receiver);
        } else {
            // If no directories selected, clear everything
            self.scanner.lock().unwrap().files.clear();
            self.scanner.lock().unwrap().tags.clear();
            self.scanner.lock().unwrap().images.clear();
            self.physics_simulator.node_positions.clear();
            self.physics_simulator.node_velocities.clear();
            self.initial_node_layout.clear();
            self.file_graph.graph.clear();
            self.file_graph.node_indices.clear();
            self.tag_graph.graph.clear();
            self.tag_graph.file_node_indices.clear();
            self.tag_graph.tag_node_indices.clear();
            self.tag_graph.image_node_indices.clear();

            self.scan_error = Some("No directories selected for scanning".to_string());
        }
    }

    fn collect_selected_paths(&self, node: &DirectoryNode, paths: &mut Vec<PathBuf>) {
        if node.selected {
            paths.push(node.path.clone());
        }
        for child in &node.children {
            self.collect_selected_paths(child, paths);
        }
    }

    fn try_load_file_content(&mut self, path: PathBuf, ctx: &egui::Context) {
        let file_extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ["png", "jpg", "jpeg", "gif", "bmp", "webp"].contains(&file_extension.as_str()) {
            match image::open(&path) {
                Ok(img) => {
                    let rgba_image = img.into_rgba8();
                    let pixels: Vec<u8> = rgba_image.as_flat_samples().as_slice().to_vec();
                    let image_size = [rgba_image.width() as _, rgba_image.height() as _];
                    let image_data = egui::ColorImage::from_rgba_unmultiplied(image_size, &pixels);
                    self.selected_image = Some(ctx.load_texture(
                        // Use ctx instead of frame.ctx()
                        path.to_string_lossy(),
                        image_data,
                        Default::default(),
                    ));
                    self.selected_file_content = None;
                }
                Err(e) => {
                    self.selected_file_content = Some(format!("Failed to load image: {}", e));
                    self.selected_image = None;
                }
            }
        } else {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    self.selected_file_content = Some(content);
                    self.selected_image = None;
                }
                Err(e) => {
                    self.selected_file_content = Some(format!("Failed to read file: {}", e));
                    self.selected_image = None;
                }
            }
        }
    }

    fn render_directory_tree_node(ui: &mut egui::Ui, node: &mut DirectoryNode) -> bool {
        let mut changed = false;
        let label = node.path.file_name().unwrap().to_string_lossy().to_string();

        ui.horizontal(|ui| {
            if ui.checkbox(&mut node.selected, "").changed() {
                changed = true;
            }

            // Clickable label for expansion/collapse
            if ui
                .add(egui::Label::new(label).sense(Sense::click()))
                .clicked()
            {
                node.expanded = !node.expanded;
                changed = true;
            }
        });

        if node.expanded {
            ui.indent("dir_indent", |ui| {
                for child in &mut node.children {
                    if Self::render_directory_tree_node(ui, child) {
                        changed = true;
                    }
                }
            });
        }

        changed
    }

    fn focus_on_node(&mut self, node_idx: NodeIndex) {
        if let Some(&node_pos) = self.physics_simulator.get_node_position(node_idx) {
            let current_center_offset = self.graph_center_offset;
            let target_center_offset = -node_pos; // Center the node at (0,0) in graph coordinates
            self.graph_center_offset = target_center_offset;
            self.graph_zoom_factor = 1.0; // Reset zoom to default
        }
    }

    fn center_graph(&mut self) {
        self.graph_center_offset = egui::Vec2::ZERO;
        self.graph_zoom_factor = 1.0;
    }

    fn perform_search(&mut self) {
        self.search_results.clear();
        self.current_search_result = 0;

        let query_lower = self.search_query.to_lowercase();
        if query_lower.is_empty() {
            return;
        }

        let graph_to_search = match self.current_graph_mode {
            GraphMode::Links => &self.file_graph.graph,
            GraphMode::Tags => &self.tag_graph.graph,
        };

        for node_idx in graph_to_search.node_indices() {
            let node_name = match &graph_to_search[node_idx] {
                GraphNode::File(s) => PathBuf::from(s)
                    .file_name()
                    .map_or_else(|| s.clone(), |os_str| os_str.to_string_lossy().into_owned()),
                GraphNode::Tag(s) => s.clone(),
            };
            if node_name.to_lowercase().contains(&query_lower) {
                self.search_results.push(node_idx);
            }
        }

        if !self.search_results.is_empty() {
            self.selected_node = Some(self.search_results[0]);
            self.focus_on_node(self.search_results[0]);
        }
    }

    fn focus_next_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        self.current_search_result = (self.current_search_result + 1) % self.search_results.len();
        self.selected_node = Some(self.search_results[self.current_search_result]);
        self.focus_on_node(self.search_results[self.current_search_result]);
    }

    fn focus_prev_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        if self.current_search_result == 0 {
            self.current_search_result = self.search_results.len() - 1;
        } else {
            self.current_search_result -= 1;
        }
        self.selected_node = Some(self.search_results[self.current_search_result]);
        self.focus_on_node(self.search_results[self.current_search_result]);
    }

    fn is_markdown_file(&self) -> bool {
        if let Some(node_idx) = self.selected_node {
            let graph = match self.current_graph_mode {
                GraphMode::Links => &self.file_graph.graph,
                GraphMode::Tags => &self.tag_graph.graph,
            };
            if let GraphNode::File(file_path_str) = &graph[node_idx] {
                return is_markdown_path(Path::new(file_path_str));
            }
        }
        false
    }

    fn is_code_file(&self) -> bool {
        if let Some(node_idx) = self.selected_node {
            let graph = match self.current_graph_mode {
                GraphMode::Links => &self.file_graph.graph,
                GraphMode::Tags => &self.tag_graph.graph,
            };
            if let GraphNode::File(file_path_str) = &graph[node_idx] {
                return is_code_path(Path::new(file_path_str));
            }
        }
        false
    }

    fn render_code_with_syntax_highlighting(&mut self, ui: &mut egui::Ui, _code_content: &str) {
        let content = if let Some(content) = &self.selected_file_content {
            content.clone()
        } else {
            return;
        };

        if let Some(node_idx) = self.selected_node {
            let graph = match self.current_graph_mode {
                GraphMode::Links => &self.file_graph.graph,
                GraphMode::Tags => &self.tag_graph.graph,
            };
            let file_path_str = if let GraphNode::File(s) = &graph[node_idx] {
                s
            } else {
                return;
            };

            let path = PathBuf::from(file_path_str);
            let lang = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            let syntax = self.get_syntax_for_language(&lang);

            if let Some(syntax_ref) = syntax {
                let mut h = HighlightLines::new(syntax_ref, *DEFAULT_THEME);
                let mut layouter = |ui: &egui::Ui, text: &str, _wrap_width: f32| {
                    let mut job = egui::text::LayoutJob::default();
                    for line in LinesWithEndings::from(text) {
                        let ranges = h.highlight_line(line, &SYNTAX_SET).unwrap();
                        for (style, text) in ranges {
                            let color = style.foreground;
                            let egui_color = egui::Color32::from_rgb(color.r, color.g, color.b);
                            job.append(
                                text,
                                0.0,
                                egui::TextFormat {
                                    font_id: egui::TextStyle::Monospace.resolve(ui.style()),
                                    color: egui_color,
                                    ..Default::default()
                                },
                            );
                        }
                    }
                    ui.fonts(|f| f.layout_job(job))
                };

                let mut text = content;
                ui.add(
                    egui::TextEdit::multiline(&mut text)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(ui.available_width())
                        .interactive(false)
                        .layouter(&mut layouter),
                );
            } else {
                let mut text = content;
                ui.add(
                    egui::TextEdit::multiline(&mut text)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(ui.available_width()),
                );
            }
        }
    }

    fn render_code_block(
        &mut self,
        ui: &mut egui::Ui,
        code_block_content: &str,
        syntax: Option<&SyntaxReference>,
    ) {
        if let Some(syntax_ref) = syntax {
            let mut h = HighlightLines::new(syntax_ref, *DEFAULT_THEME);
            let mut layouter = |ui: &egui::Ui, text: &str, _wrap_width: f32| {
                let mut job = egui::text::LayoutJob::default();
                for line in LinesWithEndings::from(text) {
                    let ranges = h.highlight_line(line, &SYNTAX_SET).unwrap();
                    for (style, text) in ranges {
                        let color = style.foreground;
                        let egui_color = egui::Color32::from_rgb(color.r, color.g, color.b);
                        job.append(
                            text,
                            0.0,
                            egui::TextFormat {
                                font_id: egui::TextStyle::Monospace.resolve(ui.style()),
                                color: egui_color,
                                ..Default::default()
                            },
                        );
                    }
                }
                ui.fonts(|f| f.layout_job(job))
            };

            let mut text = code_block_content.to_string();
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(ui.available_width())
                    .interactive(false)
                    .layouter(&mut layouter),
            );
        } else {
            let mut text = code_block_content.to_string();
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(ui.available_width()),
            );
        }
    }

    fn get_syntax_for_language(&self, lang: &str) -> Option<&SyntaxReference> {
        match lang.to_lowercase().as_str() {
            "" => Some(SYNTAX_SET.find_syntax_plain_text()),
            "python" | "py" => SYNTAX_SET.find_syntax_by_extension("py"),
            "c" | "cpp" | "h" => SYNTAX_SET.find_syntax_by_extension("c"),
            "rust" | "rs" => SYNTAX_SET.find_syntax_by_extension("rs"),
            "javascript" | "js" => SYNTAX_SET.find_syntax_by_extension("js"),
            "html" => SYNTAX_SET.find_syntax_by_extension("html"),
            "css" => SYNTAX_SET.find_syntax_by_extension("css"),
            "bash" | "sh" => SYNTAX_SET.find_syntax_by_extension("sh"),
            "markdown" | "md" => SYNTAX_SET.find_syntax_by_extension("md"),
            _ => SYNTAX_SET
                .find_syntax_by_extension(lang)
                .or_else(|| Some(SYNTAX_SET.find_syntax_plain_text())),
        }
    }
}
