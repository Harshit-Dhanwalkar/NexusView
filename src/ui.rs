use crate::file_scan;
use crate::graph;
use eframe::egui;
use petgraph::visit::EdgeRef;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(PartialEq, Eq, Clone)]
pub enum GraphViewMode {
    LinkGraph,
    TagGraph,
}

pub struct FileGraphApp {
    scanner: file_scan::FileScanner,
    graph: graph::FileGraph,
    tag_graph: graph::TagGraph,
    current_graph_mode: GraphViewMode,
    selected_node: Option<usize>,
    selected_image: Option<(PathBuf, egui::ColorImage)>,
    selected_file_content: Option<String>,
    current_dir: PathBuf,
    should_exit: bool,
    texture: Option<egui::TextureHandle>,
    tag_filter_input: String,
    node_positions: HashMap<petgraph::graph::NodeIndex, egui::Pos2>,
    zoom_factor: f32,
    pan_offset: egui::Vec2,
    show_graph_view: bool,
    is_dragging: bool,
    show_full_path: bool,
}

impl FileGraphApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, dir_path: &str) -> Self {
        let current_dir = PathBuf::from(dir_path);
        let mut scanner = file_scan::FileScanner::new(&current_dir);
        scanner.scan();

        let mut graph = graph::FileGraph::new();
        graph.build_from_scanner(&scanner);

        let mut tag_graph = graph::TagGraph::new();
        tag_graph.build_from_tags(&scanner);

        Self {
            scanner,
            graph,
            tag_graph,
            current_graph_mode: GraphViewMode::LinkGraph,
            selected_node: None,
            selected_image: None,
            selected_file_content: None,
            current_dir,
            should_exit: false,
            texture: None,
            tag_filter_input: String::new(),
            node_positions: HashMap::new(),
            zoom_factor: 1.0,
            pan_offset: egui::Vec2::ZERO,
            show_graph_view: false,
            is_dragging: false,
            show_full_path: false,
        }
    }

    fn load_image(&mut self, _ctx: &egui::Context, path: &PathBuf) -> Option<egui::ColorImage> {
        if let Ok(image_data) = std::fs::read(path) {
            if let Ok(image) = image::load_from_memory(&image_data) {
                let size = [image.width() as usize, image.height() as usize];
                let image_buffer = image.to_rgba8();
                let pixels = image_buffer.as_flat_samples();
                return Some(egui::ColorImage::from_rgba_unmultiplied(
                    size,
                    pixels.as_slice(),
                ));
            }
        }
        None
    }

    fn get_display_name(&self, path: &str) -> String {
        if self.show_full_path {
            let path_buf = PathBuf::from(path);
            if let Ok(abs_path) = path_buf.canonicalize() {
                abs_path.display().to_string()
            } else {
                path.to_string()
            }
        } else {
            Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path)
                .to_string()
        }
    }

    fn calculate_initial_node_positions(&mut self) -> egui::Rect {
        let current_graph = match self.current_graph_mode {
            GraphViewMode::LinkGraph => &self.graph.graph,
            GraphViewMode::TagGraph => &self.tag_graph.graph,
        };

        self.node_positions.clear();

        let num_nodes = current_graph.node_count();
        if num_nodes == 0 {
            return egui::Rect::NOTHING;
        }

        let nodes_per_row = (num_nodes as f32).sqrt().ceil() as usize;
        let spacing = if self.show_full_path { 250.0 } else { 150.0 };

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for (i, node_idx) in current_graph.node_indices().enumerate() {
            let col = i % nodes_per_row;
            let row = i / nodes_per_row;

            let x = col as f32 * spacing;
            let y = row as f32 * spacing;

            let pos = egui::pos2(x, y);
            self.node_positions.insert(node_idx, pos);

            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }

        egui::Rect::from_min_max(egui::pos2(min_x, min_y), egui::pos2(max_x, max_y))
    }
}

impl eframe::App for FileGraphApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(format!("File Graph: {}", self.current_dir.display()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui.button("❌ Exit").clicked() {
                        self.should_exit = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }

                    ui.add_space(20.0);

                    if ui
                        .button(if self.show_graph_view {
                            "Close Graph View"
                        } else {
                            "Open Graph View"
                        })
                        .clicked()
                    {
                        self.show_graph_view = !self.show_graph_view;
                        if self.show_graph_view {
                            self.node_positions.clear();
                            self.selected_node = None;
                            self.selected_image = None;
                            self.selected_file_content = None;
                        }
                    }
                });
            });
        });

        let mut clicked_image_path = None;

        egui::SidePanel::left("file_list_panel").show(ctx, |ui| {
            ui.collapsing("File List", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Filter by tag:");
                    ui.text_edit_singleline(&mut self.tag_filter_input);
                });

                let filter_tag_lowercase = self.tag_filter_input.to_lowercase();

                for (path, links) in &self.scanner.files {
                    let file_tags = self.scanner.tags.get(path);
                    let display_file = if filter_tag_lowercase.is_empty() {
                        true
                    } else {
                        file_tags.map_or(false, |tags| {
                            tags.iter()
                                .any(|tag| tag.to_lowercase().contains(&filter_tag_lowercase))
                        })
                    };

                    if display_file {
                        let mut button_text =
                            format!("📄 {} ({} links)", path.display(), links.len());
                        if let Some(tags) = file_tags {
                            button_text.push_str(&format!(" [{}]", tags.join(", ")));
                        }
                        if ui.button(button_text).clicked() {
                            self.selected_node = match self.current_graph_mode {
                                GraphViewMode::LinkGraph => {
                                    self.graph.node_indices().get(path).map(|i| i.index())
                                }
                                GraphViewMode::TagGraph => {
                                    self.tag_graph.node_indices.get(path).map(|i| i.index())
                                }
                            };
                            self.selected_image = None;
                            self.selected_file_content = std::fs::read_to_string(path).ok();
                            self.show_graph_view = false;
                        }
                    }
                }

                for path in &self.scanner.images {
                    if ui.button(format!("🖼️ {}", path.display())).clicked() {
                        clicked_image_path = Some(path.clone());
                        self.show_graph_view = false;
                    }
                }
            });

            if let Some(path) = clicked_image_path {
                if let Some(image) = self.load_image(ctx, &path) {
                    let texture = ctx.load_texture(
                        path.display().to_string(),
                        image.clone(),
                        Default::default(),
                    );
                    self.texture = Some(texture);
                    self.selected_image = Some((path, image));
                }
                self.selected_node = None;
                self.selected_file_content = None;
            }

            ui.separator();

            if let Some((path, _)) = &self.selected_image {
                if let Some(texture) = &self.texture {
                    ui.heading(format!("Image: {}", path.display()));
                    ui.add_space(5.0);
                    ui.image(texture);
                }
            } else if let Some(idx) = self.selected_node {
                let (graph_to_display_details, _node_indices_map) = match self.current_graph_mode {
                    GraphViewMode::LinkGraph => (&self.graph.graph, &self.graph.node_indices),
                    GraphViewMode::TagGraph => {
                        (&self.tag_graph.graph, &self.tag_graph.node_indices)
                    }
                };

                if let Some(node) = graph_to_display_details.node_indices().nth(idx) {
                    let node_path_str = &graph_to_display_details[node];
                    let node_path_buf = PathBuf::from(node_path_str);

                    ui.heading(format!("Selected File: {}", node_path_buf.display()));

                    if let Some(tags) = self.scanner.tags.get(&node_path_buf) {
                        ui.label(format!("Tags: {}", tags.join(", ")));
                    }

                    if let Some(content) = &self.selected_file_content {
                        ui.separator();
                        ui.heading("File Content:");
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut content.as_str())
                                    .desired_width(f32::INFINITY),
                            );
                        });
                    }

                    ui.separator();
                    ui.heading("Connections:");
                    match self.current_graph_mode {
                        GraphViewMode::LinkGraph => {
                            ui.label("Links to this file:");
                            for edge in graph_to_display_details
                                .edges_directed(node, petgraph::Direction::Incoming)
                            {
                                ui.label(format!("← {}", graph_to_display_details[edge.source()]));
                            }

                            ui.label("Links from this file:");
                            for edge in graph_to_display_details
                                .edges_directed(node, petgraph::Direction::Outgoing)
                            {
                                ui.label(format!("→ {}", graph_to_display_details[edge.target()]));
                            }
                        }
                        GraphViewMode::TagGraph => {
                            ui.label("Files sharing tags with this file:");
                            for edge in graph_to_display_details
                                .edges_directed(node, petgraph::Direction::Incoming)
                            {
                                ui.label(format!("↔ {}", graph_to_display_details[edge.source()]));
                            }
                            for edge in graph_to_display_details
                                .edges_directed(node, petgraph::Direction::Outgoing)
                            {
                                if edge.source() != node {
                                    ui.label(format!(
                                        "↔ {}",
                                        graph_to_display_details[edge.target()]
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.show_graph_view {
                let mut changed_mode = false;

                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.label("Graph Type:");

                    if ui
                        .radio_value(
                            &mut self.current_graph_mode,
                            GraphViewMode::LinkGraph,
                            "Links",
                        )
                        .clicked()
                    {
                        changed_mode = true;
                    }
                    if ui
                        .radio_value(
                            &mut self.current_graph_mode,
                            GraphViewMode::TagGraph,
                            "Tags",
                        )
                        .clicked()
                    {
                        changed_mode = true;
                    }

                    ui.add_space(20.0);
                    if ui
                        .button(if self.show_full_path {
                            "Show Filenames Only"
                        } else {
                            "Show Absolute Paths"
                        })
                        .clicked()
                    {
                        self.show_full_path = !self.show_full_path;
                    }
                });

                ui.separator();

                let graph_rect = ui.available_rect_before_wrap();

                if changed_mode || self.node_positions.is_empty() {
                    let layout_rect = self.calculate_initial_node_positions();

                    if layout_rect == egui::Rect::NOTHING {
                        self.pan_offset = egui::Vec2::ZERO;
                        self.zoom_factor = 1.0;
                    } else {
                        let target_center = graph_rect.center();
                        let layout_center = layout_rect.center();
                        self.pan_offset = target_center.to_vec2() - layout_center.to_vec2();
                        self.zoom_factor = 1.0;
                    }
                }

                let graph_response = ui.allocate_rect(graph_rect, egui::Sense::drag());

                if graph_response.dragged() {
                    self.pan_offset += graph_response.drag_delta();
                }

                let zoom_delta = ctx.input(|i| i.zoom_delta());
                if zoom_delta != 1.0 && graph_response.hovered() {
                    let old_zoom = self.zoom_factor;
                    self.zoom_factor *= zoom_delta;
                    self.zoom_factor = self.zoom_factor.clamp(0.1, 5.0);

                    if let Some(mouse_screen_pos) = ctx.pointer_hover_pos() {
                        let mouse_relative_to_graph_rect =
                            mouse_screen_pos - graph_rect.min.to_vec2();
                        let mouse_in_graph_space_old =
                            (mouse_relative_to_graph_rect - self.pan_offset) / old_zoom;
                        let mouse_in_graph_space_new =
                            (mouse_relative_to_graph_rect - self.pan_offset) / self.zoom_factor;
                        self.pan_offset += (mouse_in_graph_space_new - mouse_in_graph_space_old)
                            * self.zoom_factor;
                    }
                }

                let painter = ui.painter();
                let current_graph = match self.current_graph_mode {
                    GraphViewMode::LinkGraph => &self.graph.graph,
                    GraphViewMode::TagGraph => &self.tag_graph.graph,
                };

                let transform_pos = |p: egui::Pos2| {
                    (p.to_vec2() * self.zoom_factor + self.pan_offset + graph_rect.min.to_vec2())
                        .to_pos2()
                };

                // Draw Edges first
                for edge in current_graph.raw_edges() {
                    if let (Some(&start_pos), Some(&end_pos)) = (
                        self.node_positions.get(&edge.source()),
                        self.node_positions.get(&edge.target()),
                    ) {
                        painter.line_segment(
                            [transform_pos(start_pos), transform_pos(end_pos)],
                            egui::Stroke::new(1.0, egui::Color32::GRAY),
                        );
                    }
                }

                // Draw Nodes and Labels
                let node_radius = 20.0 * self.zoom_factor;
                for node_idx in current_graph.node_indices() {
                    if let Some(&center_pos) = self.node_positions.get(&node_idx) {
                        let actual_center_pos = transform_pos(center_pos);
                        let node_name = &current_graph[node_idx];
                        let display_name = self.get_display_name(node_name);

                        let font_size = 10.0 * self.zoom_factor;
                        let font_id = egui::FontId::proportional(font_size);
                        let galley = painter.layout_no_wrap(
                            display_name.clone(),
                            font_id,
                            egui::Color32::WHITE,
                        );

                        let (node_center, text_pos) = if self.show_full_path {
                            let node_center = actual_center_pos;
                            let text_pos = node_center + egui::vec2(0.0, node_radius + 2.0);
                            (node_center, text_pos)
                        } else {
                            (actual_center_pos, actual_center_pos)
                        };

                        let node_color = if self.selected_node == Some(node_idx.index()) {
                            egui::Color32::LIGHT_BLUE
                        } else {
                            egui::Color32::BLUE
                        };
                        painter.circle_filled(node_center, node_radius, node_color);

                        if self.show_full_path {
                            painter.rect_filled(
                                egui::Rect::from_center_size(
                                    text_pos + galley.size() * 0.5,
                                    galley.size() + egui::vec2(4.0, 2.0),
                                ),
                                2.0,
                                egui::Color32::from_black_alpha(100),
                            );
                        }
                        painter.galley(text_pos, galley.clone(), egui::Color32::WHITE);

                        let node_rect = if self.show_full_path {
                            egui::Rect::from_center_size(
                                node_center,
                                egui::vec2(
                                    galley.size().x.max(node_radius * 2.0),
                                    node_radius * 2.0 + galley.size().y,
                                ),
                            )
                        } else {
                            egui::Rect::from_center_size(
                                node_center,
                                egui::vec2(node_radius * 2.0, node_radius * 2.0),
                            )
                        };

                        let node_response = ui.interact(
                            node_rect,
                            ui.id().with(node_idx.index()),
                            egui::Sense::click(),
                        );

                        if node_response.clicked() {
                            self.selected_node = Some(node_idx.index());
                            self.selected_image = None;
                            let path_buf = PathBuf::from(node_name);
                            self.selected_file_content = std::fs::read_to_string(&path_buf).ok();
                        }

                        if node_response.hovered() {
                            egui::show_tooltip_at_pointer(
                                ctx,
                                egui::LayerId::background(),
                                egui::Id::new("node_tooltip"),
                                |ui| {
                                    ui.label(egui::WidgetText::from(node_name));
                                },
                            );
                        }
                    }
                }
            }
        });

        if self.should_exit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
