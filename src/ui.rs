// src/ui.rs
use eframe::{App, egui};
use egui_commonmark::CommonMarkViewer;
use petgraph::stable_graph::NodeIndex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::file_scan::FileScanner;
use crate::graph::{FileGraph, GraphNode, TagGraph};
use crate::physics_nodes::PhysicsSimulator;
use egui::{Color32, Sense, Stroke, pos2, vec2};
use petgraph::visit::EdgeRef;
use rand::Rng;

#[derive(PartialEq)]
enum GraphMode {
    Links,
    Tags,
}

pub struct FileGraphApp {
    scan_dir: PathBuf,
    scanner: Arc<Mutex<FileScanner>>,
    file_graph: FileGraph,
    tag_graph: TagGraph,
    current_graph_mode: GraphMode,
    show_full_paths: bool,
    physics_simulator: PhysicsSimulator,
    is_scanning: bool,
    scan_error: Option<String>,
    selected_node: Option<petgraph::graph::NodeIndex>,
    selected_file_content: Option<String>,
    selected_image: Option<egui::TextureHandle>,
    tag_filter_input: String,
    initial_node_layout: HashMap<petgraph::graph::NodeIndex, egui::Vec2>,
    graph_center_offset: egui::Vec2,
    graph_zoom_factor: f32,
    dragged_node: Option<petgraph::graph::NodeIndex>,
    last_drag_pos: Option<egui::Pos2>,
    current_directory_label: String,
    show_images: bool,
    // show_orphans: bool,
    graph_rect: egui::Rect,
    markdown_cache: egui_commonmark::CommonMarkCache,
    scan_progress: f32,
    scan_status: String,
    graph_build_progress: f32,
    graph_build_status: String,
    scan_progress_receiver: Option<std::sync::mpsc::Receiver<(f32, String)>>,
    search_query: String,
    search_results: Vec<NodeIndex>,
    current_search_result: usize,
    open_menu_on_node: Option<NodeIndex>,
    right_click_menu_pos: Option<egui::Pos2>,
    menu_open: bool,
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
            self.file_graph.build_from_scanner(&scanner_locked);
            self.graph_build_progress = 0.5;
            self.graph_build_status = "Building tag graph...".to_string();
            ctx.request_repaint();
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
                // ui.checkbox(&mut self.show_orphans, "Show Orphans");
                ui.checkbox(&mut self.show_images, "Show Images");

                ui.separator();

                ui.label("Filter Tags:");
                ui.text_edit_singleline(&mut self.tag_filter_input);

                if ui.button("Rescan Directory").clicked() && !self.is_scanning {
                    self.is_scanning = true;
                    self.scan_progress = 0.0;
                    self.scan_status = "Starting scan...".to_string();

                    let scanner_arc_clone = self.scanner.clone();
                    let scan_dir_clone = self.scan_dir.clone();
                    let (progress_sender, progress_receiver) = std::sync::mpsc::channel();

                    thread::spawn(move || {
                        let mut scanner = scanner_arc_clone.lock().unwrap();
                        if let Err(e) =
                            scanner.scan_directory_with_progress(&scan_dir_clone, progress_sender)
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
                                egui::Slider::new(&mut self.physics_simulator.damping, 0.0..=0.9)
                                    .text("Damping"),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.physics_simulator.time_step, 0.1..=1.0)
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
                                egui::Slider::new(&mut self.physics_simulator.friction, 0.0..=0.9)
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

        egui::CentralPanel::default().show(ctx, |ui| {
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
                                egui::ProgressBar::new(self.graph_build_progress).show_percentage(),
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
                        let nodes: Vec<_> =
                            self.file_graph.node_indices.values().cloned().collect();

                        let edges: Vec<_> = self
                            .file_graph
                            .graph
                            .edge_references()
                            .map(|e| (e.source(), e.target()))
                            .collect();
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
            }

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

                    let stroke = Stroke::new(1.0, Color32::GRAY);
                    painter.line_segment([start_screen_pos, end_screen_pos], stroke);

                    let vec_between = end_screen_pos - start_screen_pos;
                    let end_point_for_arrow = start_screen_pos + vec_between * 0.9;

                    let dir = vec_between.normalized();
                    let arrow_size = 8.0;

                    let arrow_tip1 = end_point_for_arrow - rotate_vec2(dir, 0.5) * arrow_size;
                    let arrow_tip2 = end_point_for_arrow - rotate_vec2(dir, -0.5) * arrow_size;

                    painter.line_segment([end_point_for_arrow, arrow_tip1], stroke);
                    painter.line_segment([end_point_for_arrow, arrow_tip2], stroke);
                }
            }

            for &node_idx in &nodes_to_draw {
                if let Some(node_pos_vec2) =
                    self.physics_simulator.get_node_position(node_idx).cloned()
                {
                    let screen_pos = to_screen.transform_pos(pos2(
                        node_pos_vec2.x * self.graph_zoom_factor + self.graph_center_offset.x,
                        node_pos_vec2.y * self.graph_zoom_factor + self.graph_center_offset.y,
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

                    let node_radius = 15.0 * self.graph_zoom_factor;
                    let node_color = if Some(node_idx) == self.selected_node {
                        Color32::from_rgb(255, 100, 100)
                    } else if self.search_results.contains(&node_idx) {
                        Color32::from_rgb(100, 255, 100)
                    } else {
                        match self.current_graph_mode {
                            GraphMode::Links => match &self.file_graph.graph[node_idx] {
                                GraphNode::File(path) => {
                                    let path = Path::new(path);
                                    let is_image = path
                                        .extension()
                                        .map(|ext| {
                                            let ext = ext.to_string_lossy().to_lowercase();
                                            ["png", "jpg", "jpeg", "gif", "bmp", "webp", "ind"]
                                                .contains(&ext.as_str())
                                        })
                                        .unwrap_or(false);
                                    if is_image {
                                        Color32::from_rgb(255, 165, 0)
                                    } else {
                                        Color32::BLUE
                                    }
                                }
                                GraphNode::Tag(_) => Color32::GREEN,
                            },
                            GraphMode::Tags => match &self.tag_graph.graph[node_idx] {
                                GraphNode::File(path) => {
                                    let scanner_locked = self.scanner.lock().unwrap();
                                    let has_tags =
                                        scanner_locked.tags.contains_key(Path::new(path));
                                    let is_image = Path::new(path)
                                        .extension()
                                        .map(|ext| {
                                            let ext = ext.to_string_lossy().to_lowercase();
                                            ["png", "jpg", "jpeg", "gif", "bmp", "webp"]
                                                .contains(&ext.as_str())
                                        })
                                        .unwrap_or(false);

                                    if is_image {
                                        Color32::from_rgb(255, 165, 0)
                                    } else if has_tags {
                                        Color32::BLUE
                                    } else {
                                        Color32::GRAY
                                    }
                                }
                                GraphNode::Tag(_) => Color32::GREEN,
                            },
                        }
                    };

                    // Pulse effect for selected node
                    let pulse = if Some(node_idx) == self.selected_node {
                        ctx.input(|i| i.time as f32).sin().abs() * 0.2 + 0.8
                    } else {
                        1.0
                    };

                    painter.circle_filled(screen_pos, node_radius * pulse, node_color);

                    let display_name = if self.show_full_paths {
                        node_name.clone()
                    } else {
                        PathBuf::from(&node_name).file_name().map_or_else(
                            || node_name.clone(),
                            |os_str| os_str.to_string_lossy().into_owned(),
                        )
                    };

                    let font_id = egui::TextStyle::Body.resolve(ui.style());
                    let text_galley =
                        ui.fonts(|f| f.layout_no_wrap(display_name, font_id, Color32::WHITE));

                    let text_pos =
                        screen_pos + vec2(-text_galley.size().x / 2.0, node_radius + 5.0);
                    painter.galley(text_pos, text_galley.clone(), Color32::WHITE);

                    let node_rect = if text_galley.size().y > 0.0 {
                        egui::Rect::from_center_size(
                            screen_pos,
                            egui::vec2(node_radius * 2.0, node_radius * 2.0 + text_galley.size().y),
                        )
                    } else {
                        egui::Rect::from_center_size(
                            screen_pos,
                            egui::vec2(node_radius * 2.0, node_radius * 2.0),
                        )
                    };

                    let node_response =
                        ui.interact(node_rect, ui.id().with(node_idx), Sense::click_and_drag());

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

                    // Hover effect for absolute path
                    if node_response.hovered() {
                        let full_name = match self.current_graph_mode {
                            GraphMode::Links => match &self.file_graph.graph[node_idx] {
                                GraphNode::File(file_path_str) => file_path_str.clone(),
                                GraphNode::Tag(tag_name) => format!("Tag: #{}", tag_name),
                            },
                            GraphMode::Tags => match &self.tag_graph.graph[node_idx] {
                                GraphNode::File(file_path_str) => file_path_str.clone(),
                                GraphNode::Tag(tag_name) => format!("Tag: #{}", tag_name),
                            },
                        };
                        egui::show_tooltip_at_pointer(
                            ctx,
                            egui::LayerId::background(),
                            egui::Id::new("node_tooltip").with(node_idx),
                            |ui| {
                                ui.label(full_name);
                                if ui.button("Focus").clicked() {
                                    self.focus_on_node(node_idx);
                                }
                            },
                        );
                    }

                    // Left-click to select node and show content
                    if node_response.clicked() {
                        self.selected_node = Some(node_idx);
                        self.selected_image = None;
                        self.selected_file_content = None;

                        let path_buf = match self.current_graph_mode {
                            GraphMode::Links => match &self.file_graph.graph[node_idx] {
                                GraphNode::File(s) => PathBuf::from(s),
                                GraphNode::Tag(_) => PathBuf::new(),
                            },
                            GraphMode::Tags => match &self.tag_graph.graph[node_idx] {
                                GraphNode::File(s) => PathBuf::from(s),
                                GraphNode::Tag(_) => PathBuf::new(),
                            },
                        };

                        if path_buf.exists() && path_buf.is_file() {
                            if let Some(ext) = path_buf.extension().and_then(|e| e.to_str()) {
                                let image_extensions = ["png", "jpg", "jpeg", "gif", "bmp", "webp"];
                                if image_extensions.contains(&ext.to_lowercase().as_str()) {
                                    match image::open(&path_buf) {
                                        Ok(img) => {
                                            let size = [img.width() as _, img.height() as _];
                                            let image_buffer = img.to_rgba8();
                                            let pixels = image_buffer.as_flat_samples();
                                            let color_image =
                                                egui::ColorImage::from_rgba_unmultiplied(
                                                    size,
                                                    pixels.as_slice(),
                                                );
                                            self.selected_image = Some(ctx.load_texture(
                                                path_buf.to_string_lossy(),
                                                color_image,
                                                egui::TextureOptions::LINEAR,
                                            ));
                                        }
                                        Err(e) => {
                                            eprintln!("Error loading image: {}", e);
                                            self.selected_image = None;
                                        }
                                    }
                                } else {
                                    self.selected_file_content =
                                        std::fs::read_to_string(&path_buf).ok();
                                }
                            } else {
                                self.selected_file_content =
                                    std::fs::read_to_string(&path_buf).ok();
                            }
                        }
                    }

                    // Right-click to open menu
                    if node_response.clicked_by(egui::PointerButton::Secondary) {
                        self.open_menu_on_node = Some(node_idx);
                        // Store the current pointer position for the menu
                        self.right_click_menu_pos = ctx.input(|i| i.pointer.interact_pos());
                        self.menu_open = true;
                    }
                }
            }

            // Render the custom right-click menu as an egui::Window
            if let Some(menu_node_idx) = self.open_menu_on_node {
                if let Some(menu_pos) = self.right_click_menu_pos {
                    // Use the stored mouse position
                    let mut is_open = true;
                    let window_response = egui::Window::new("Node Actions")
                        .id(egui::Id::new("right_click_node_menu").with(menu_node_idx))
                        .default_pos(menu_pos)
                        .collapsible(false)
                        .resizable(false)
                        .default_width(200.0)
                        .show(ctx, |ui| {
                            let full_name_for_menu = match self.current_graph_mode {
                                GraphMode::Links => match &self.file_graph.graph[menu_node_idx] {
                                    GraphNode::File(file_path_str) => file_path_str.clone(),
                                    GraphNode::Tag(tag_name) => format!("Tag: #{}", tag_name),
                                },
                                GraphMode::Tags => match &self.tag_graph.graph[menu_node_idx] {
                                    GraphNode::File(file_path_str) => file_path_str.clone(),
                                    GraphNode::Tag(tag_name) => format!("Tag: #{}", tag_name),
                                },
                            };
                            ui.label(full_name_for_menu);
                            ui.separator();

                            let path_buf_option = match self.current_graph_mode {
                                GraphMode::Links => match &self.file_graph.graph[menu_node_idx] {
                                    GraphNode::File(s) => Some(PathBuf::from(s)),
                                    GraphNode::Tag(_) => None,
                                },
                                GraphMode::Tags => match &self.tag_graph.graph[menu_node_idx] {
                                    GraphNode::File(s) => Some(PathBuf::from(s)),
                                    GraphNode::Tag(_) => None,
                                },
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
                                        is_open = false;
                                    }
                                    if ui.button("Copy Path").clicked() {
                                        ctx.copy_text(path_buf.to_string_lossy().to_string());
                                        is_open = false;
                                    }
                                }
                            }
                        })
                        .unwrap();

                    if !self.menu_open && window_response.response.clicked_elsewhere() {
                        is_open = false;
                    }
                    self.menu_open = false;

                    if !is_open {
                        self.open_menu_on_node = None;
                        self.right_click_menu_pos = None;
                    }
                }
            }
        });

        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            ui.heading("Content Preview");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some(content) = &self.selected_file_content {
                    // Check if this is a markdown file
                    let is_markdown = if let Some(node_idx) = self.selected_node {
                        let path_str = match self.current_graph_mode {
                            GraphMode::Links => match &self.file_graph.graph[node_idx] {
                                GraphNode::File(s) => s,
                                GraphNode::Tag(_) => "",
                            },
                            GraphMode::Tags => match &self.tag_graph.graph[node_idx] {
                                GraphNode::File(s) => s,
                                GraphNode::Tag(_) => "",
                            },
                        };
                        Path::new(path_str)
                            .extension()
                            .map(|ext| ext == "md" || ext == "markdown")
                            .unwrap_or(false)
                    } else {
                        false
                    };

                    if is_markdown {
                        CommonMarkViewer::new().show(ui, &mut self.markdown_cache, content);
                    } else {
                        // Regular text file display
                        let mut text = content.clone();
                        ui.add(
                            egui::TextEdit::multiline(&mut text)
                                .desired_width(ui.available_width()),
                        );
                    }
                } else if let Some(texture) = &self.selected_image {
                    // image preview
                    ui.vertical_centered(|ui| {
                        let available_width = ui.available_width() - 20.0;
                        let img_size = texture.size_vec2();
                        let ratio = img_size.y / img_size.x;
                        let desired_height = available_width * ratio;

                        // Show image dimensions
                        ui.label(format!(
                            "Image: {}×{}",
                            img_size.x as i32, img_size.y as i32
                        ));

                        // Display the image with maximum width while maintaining aspect ratio
                        ui.image(texture);
                    });
                } else {
                    ui.label("Select a file or image to preview its content.");
                }
            });
        });

        ctx.request_repaint();
    }
}

impl FileGraphApp {
    fn perform_search(&mut self) {
        self.search_results.clear();
        self.current_search_result = 0;

        let query = self.search_query.to_lowercase();
        if query.is_empty() {
            return;
        }

        let scanner = self.scanner.lock().unwrap();
        let graph = match self.current_graph_mode {
            GraphMode::Links => &self.file_graph.graph,
            GraphMode::Tags => &self.tag_graph.graph,
        };

        for node_idx in graph.node_indices() {
            match &graph[node_idx] {
                GraphNode::File(path) => {
                    // Search in filename
                    if path.to_lowercase().contains(&query) {
                        self.search_results.push(node_idx);
                        continue;
                    }

                    // Search in file content
                    if let Ok(content) = std::fs::read_to_string(path) {
                        if content.to_lowercase().contains(&query) {
                            self.search_results.push(node_idx);
                        }
                    }
                }
                GraphNode::Tag(tag) => {
                    if tag.to_lowercase().contains(&query) {
                        self.search_results.push(node_idx);
                    }
                }
            }
        }
    }

    fn focus_next_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        self.current_search_result = (self.current_search_result + 1) % self.search_results.len();
        self.focus_on_node(self.search_results[self.current_search_result]);
    }

    fn focus_prev_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        self.current_search_result = (self.current_search_result + self.search_results.len() - 1)
            % self.search_results.len();
        self.focus_on_node(self.search_results[self.current_search_result]);
    }

    fn focus_on_node(&mut self, node_idx: petgraph::graph::NodeIndex) {
        if let Some(node_pos) = self.physics_simulator.get_node_position(node_idx) {
            // Calculate the offset needed to center focused node
            let screen_center = egui::Vec2::new(
                self.graph_rect.width() / 2.0,
                self.graph_rect.height() / 2.0,
            );

            self.graph_center_offset = screen_center - *node_pos;
            self.graph_zoom_factor = 1.5;
        }
    }

    fn center_graph(&mut self) {
        self.graph_center_offset = egui::Vec2::ZERO;
        self.graph_zoom_factor = 1.0;
    }

    pub fn new(scan_dir: PathBuf) -> Self {
        let current_dir = scan_dir.join("dummy_dir");
        if !current_dir.exists() {
            std::fs::create_dir_all(&current_dir).expect("Failed to create dummy_dir");
        }

        let current_directory_label = format!("{}", current_dir.display());

        let mut scanner = FileScanner::new(&current_dir);
        let (sender, _receiver) = std::sync::mpsc::channel();
        if let Err(e) = scanner.scan_directory_with_progress(&current_dir, sender) {
            eprintln!("Initial scan error: {}", e);
        }

        let mut graph = FileGraph::new();
        graph.build_from_scanner(&scanner);

        let mut tag_graph = TagGraph::new();
        tag_graph.build_from_tags(&scanner);

        let mut physics_simulator = PhysicsSimulator::new();
        let mut initial_node_layout = HashMap::new();
        let mut rng = rand::thread_rng();

        for &node_idx in graph.node_indices.values() {
            let random_pos = egui::vec2(rng.gen_range(-100.0..100.0), rng.gen_range(-100.0..100.0));
            physics_simulator
                .node_positions
                .insert(node_idx, random_pos);
            initial_node_layout.insert(node_idx, random_pos);
        }

        for &node_idx in tag_graph.file_node_indices.values() {
            if !physics_simulator.node_positions.contains_key(&node_idx) {
                let random_pos =
                    egui::vec2(rng.gen_range(-100.0..100.0), rng.gen_range(-100.0..100.0));
                physics_simulator
                    .node_positions
                    .insert(node_idx, random_pos);
                initial_node_layout.insert(node_idx, random_pos);
            }
        }

        for &node_idx in tag_graph.tag_node_indices.values() {
            if !physics_simulator.node_positions.contains_key(&node_idx) {
                let random_pos =
                    egui::vec2(rng.gen_range(-100.0..100.0), rng.gen_range(-100.0..100.0));
                physics_simulator
                    .node_positions
                    .insert(node_idx, random_pos);
                initial_node_layout.insert(node_idx, random_pos);
            }
        }

        physics_simulator.initialize_velocities();

        Self {
            scan_dir: current_dir,
            scanner: Arc::new(Mutex::new(scanner)),
            file_graph: graph,
            tag_graph,
            current_graph_mode: GraphMode::Links,
            selected_node: None,
            selected_image: None,
            selected_file_content: None,
            tag_filter_input: String::new(),
            physics_simulator,
            initial_node_layout,
            graph_center_offset: egui::Vec2::ZERO,
            graph_zoom_factor: 1.0,
            show_full_paths: false,
            is_scanning: false,
            scan_error: None,
            dragged_node: None,
            last_drag_pos: None,
            current_directory_label,
            show_images: true,
            // show_orphans: true,
            graph_rect: egui::Rect::NOTHING,
            markdown_cache: egui_commonmark::CommonMarkCache::default(),
            scan_progress: 0.0,
            scan_status: String::new(),
            graph_build_progress: 0.0,
            graph_build_status: String::new(),
            scan_progress_receiver: None,
            search_query: String::new(),
            search_results: Vec::new(),
            current_search_result: 0,
            open_menu_on_node: None,
            right_click_menu_pos: None,
            menu_open: false,
        }
    }
}

fn rotate_vec2(vec: egui::Vec2, angle: f32) -> egui::Vec2 {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    egui::vec2(vec.x * cos_a - vec.y * sin_a, vec.x * sin_a + vec.y * cos_a)
}
