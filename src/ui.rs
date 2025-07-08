// src/ui.rs
use eframe::{App, egui};
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
    show_orphans: bool,
    graph_rect: egui::Rect,
}

impl App for FileGraphApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
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
                ui.checkbox(&mut self.show_orphans, "Show Orphans");
                ui.checkbox(&mut self.show_images, "Show Images");

                ui.separator();

                ui.label("Filter Tags:");
                ui.text_edit_singleline(&mut self.tag_filter_input);

                if ui.button("Rescan Directory").clicked() && !self.is_scanning {
                    self.is_scanning = true;
                    self.scan_error = None;
                    self.selected_node = None;
                    self.selected_file_content = None;
                    self.selected_image = None;

                    let scanner_arc_clone = self.scanner.clone();
                    let scan_dir_clone = self.scan_dir.clone();
                    thread::spawn(move || {
                        let mut scanner = scanner_arc_clone.lock().unwrap();
                        if let Err(e) = scanner.scan_directory(&scan_dir_clone) {
                            eprintln!("Error during scan: {}", e);
                        }
                    });
                }

                if self.is_scanning {
                    ui.label("Scanning...");
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
                        let nodes: Vec<_> = self
                            .file_graph
                            .node_indices
                            .values()
                            .filter(|&&idx| {
                                if !self.show_orphans {
                                    self.file_graph.graph.neighbors(idx).count() > 0
                                } else {
                                    true
                                }
                            })
                            .cloned()
                            .collect();

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

                        if self.show_orphans {
                            nodes.extend(self.tag_graph.file_node_indices.values());
                        }

                        for (file_path, &file_node_idx) in &self.tag_graph.file_node_indices {
                            if let Some(tags_for_file) = scanner_locked.tags.get(file_path) {
                                if tags_for_file
                                    .iter()
                                    .any(|tag| filtered_tag_nodes.contains_key(tag))
                                {
                                    nodes.push(file_node_idx);
                                }
                            }
                        }

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

            // let is_orphan = match self.current_graph_mode {
            //     GraphMode::Links => false,
            //     GraphMode::Tags => {
            //         let path = match &self.tag_graph.graph[node_idx] {
            //             GraphNode::File(p) => p,
            //             GraphNode::Tag(_) => continue,
            //         };
            //         let scanner_locked = self.scanner.lock().unwrap();
            //         !scanner_locked.tags.contains_key(Path::new(path))
            //     }
            // };

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
                                    let is_orphan =
                                        !scanner_locked.tags.contains_key(Path::new(path));
                                    let is_image = Path::new(path)
                                        .extension()
                                        .map(|ext| {
                                            let ext = ext.to_string_lossy().to_lowercase();
                                            ["png", "jpg", "jpeg", "gif", "bmp", "webp"]
                                                .contains(&ext.as_str())
                                        })
                                        .unwrap_or(false);

                                    if is_orphan {
                                        Color32::GRAY
                                    } else if is_image {
                                        Color32::from_rgb(255, 165, 0)
                                    } else {
                                        Color32::BLUE
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

                    if node_response.hovered() {
                        let node_idx_copy = node_idx;
                        egui::show_tooltip_at_pointer(
                            ctx,
                            egui::LayerId::background(),
                            egui::Id::new("node_tooltip"),
                            |ui| {
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
                                ui.label(full_name);

                                if ui.button("Focus").clicked() {
                                    self.focus_on_node(node_idx);
                                }
                            },
                        );
                    }
                }
            }
        });

        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            ui.heading("Content Preview");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some(content) = &mut self.selected_file_content {
                    let mut text = content.clone();
                    ui.add(
                        egui::TextEdit::multiline(&mut text).desired_width(ui.available_width()),
                    );
                    if text != *content {
                        *content = text;
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
        if let Err(e) = scanner.scan_directory(&current_dir) {
            eprintln!("Initial scan error: {}", e);
        }

        let mut graph = FileGraph::new();
        graph.build_from_scanner(&scanner);

        let mut tag_graph = TagGraph::new();
        tag_graph.build_from_tags(&scanner);

        let mut physics_simulator = PhysicsSimulator::new();
        let mut initial_node_layout = HashMap::new();
        let mut rng = rand::rng();

        for &node_idx in graph.node_indices.values() {
            let random_pos = egui::vec2(
                rng.random_range(-100.0..100.0),
                rng.random_range(-100.0..100.0),
            );
            physics_simulator
                .node_positions
                .insert(node_idx, random_pos);
            initial_node_layout.insert(node_idx, random_pos);
        }

        for &node_idx in tag_graph.file_node_indices.values() {
            if !physics_simulator.node_positions.contains_key(&node_idx) {
                let random_pos = egui::vec2(
                    rng.random_range(-100.0..100.0),
                    rng.random_range(-100.0..100.0),
                );
                physics_simulator
                    .node_positions
                    .insert(node_idx, random_pos);
                initial_node_layout.insert(node_idx, random_pos);
            }
        }

        for &node_idx in tag_graph.tag_node_indices.values() {
            if !physics_simulator.node_positions.contains_key(&node_idx) {
                let random_pos = egui::vec2(
                    rng.random_range(-100.0..100.0),
                    rng.random_range(-100.0..100.0),
                );
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
            show_orphans: true,
            graph_rect: egui::Rect::NOTHING,
        }
    }
}

fn rotate_vec2(vec: egui::Vec2, angle: f32) -> egui::Vec2 {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    egui::vec2(vec.x * cos_a - vec.y * sin_a, vec.x * sin_a + vec.y * cos_a)
}
