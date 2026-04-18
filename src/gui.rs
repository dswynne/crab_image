// Standard
use std::env;

// External
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Sense, Stroke, Vec2};

// Local
use crate::equalize;
use crate::util;

pub struct FlowApp {
    nodes: Vec<Node>,
    connections: Vec<Connection>,
    pending_connection: Option<PortRef>,
    image_path: String,
    available_files: Vec<String>,
    status: String,
    logs: String,
    selected_node: Option<usize>,
    next_node_id: usize,
}

struct Node {
    id: usize,
    kind: NodeKind,
    pos: Pos2,
    size: Vec2,
    params: NodeParams,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    LoadImage,
    LogEqualize,
    PowerLawEqualize,
    Display,
}

#[derive(Clone, Copy)]
enum NodeParams {
    None,
    LogEqualize { c: f32 },
    PowerLawEqualize { c: f32, g: f32 },
}

struct Connection {
    from: usize,
    to: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PortKind {
    #[allow(dead_code)]
    Input,
    Output,
}

#[derive(Clone, Copy)]
struct PortRef {
    node_id: usize,
    kind: PortKind,
}

impl Node {
    fn rect(&self) -> Rect {
        Rect::from_min_size(self.pos, self.size)
    }

    fn input_point(&self) -> Pos2 {
        Pos2::new(self.pos.x - 12.0, self.pos.y + self.size.y * 0.5)
    }

    fn output_point(&self) -> Pos2 {
        Pos2::new(self.pos.x + self.size.x + 12.0, self.pos.y + self.size.y * 0.5)
    }
}

impl NodeKind {
    fn title(&self) -> &'static str {
        match self {
            NodeKind::LoadImage => "Load Image",
            NodeKind::LogEqualize => "Log Equalize",
            NodeKind::PowerLawEqualize => "Power Law Equalize",
            NodeKind::Display => "Display Result",
        }
    }

    fn has_input(&self) -> bool {
        !matches!(self, NodeKind::LoadImage)
    }

    fn has_output(&self) -> bool {
        !matches!(self, NodeKind::Display)
    }

    fn description(&self) -> &'static str {
        match self {
            NodeKind::LoadImage => "Loads a grayscale image from disk.",
            NodeKind::LogEqualize => "Applies log transformation to the input image.",
            NodeKind::PowerLawEqualize => "Applies gamma correction to the input image.",
            NodeKind::Display => "Shows a short summary of the result.",
        }
    }
}

impl Default for FlowApp {
    fn default() -> Self {
        Self {
            nodes: vec![
                Node {
                    id: 1,
                    kind: NodeKind::LoadImage,
                    pos: Pos2::new(60.0, 150.0),
                    size: Vec2::new(220.0, 150.0),
                    params: NodeParams::None,
                },
                Node {
                    id: 2,
                    kind: NodeKind::LogEqualize,
                    pos: Pos2::new(360.0, 150.0),
                    size: Vec2::new(220.0, 150.0),
                    params: NodeParams::LogEqualize { c: 1.0 },
                },
                Node {
                    id: 3,
                    kind: NodeKind::Display,
                    pos: Pos2::new(660.0, 150.0),
                    size: Vec2::new(220.0, 150.0),
                    params: NodeParams::None,
                },
            ],
            connections: vec![Connection { from: 1, to: 2 }, Connection { from: 2, to: 3 }],
            pending_connection: None,
            image_path: "data/lena.tif".to_owned(),
            available_files: Vec::new(),
            status: "Ready".to_owned(),
            logs: "Use the buttons below to add nodes, then connect them by clicking ports.".to_owned(),
            selected_node: None,
            next_node_id: 4,
        }
    }
}

impl eframe::App for FlowApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // Update available files
        self.available_files.clear();
        if let Ok(entries) = std::fs::read_dir("data") {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.ends_with(".tif") || file_name.ends_with(".png") || file_name.ends_with(".jpg") || file_name.ends_with(".jpeg") {
                        self.available_files.push(format!("data/{}", file_name));
                    }
                }
            }
        }
        self.available_files.sort();

        // Ensure current image_path is valid
        if !self.available_files.contains(&self.image_path) {
            if let Some(first) = self.available_files.first() {
                self.image_path = first.clone();
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("crab_image Flow GUI");
                ui.separator();
                egui::ComboBox::from_label("Image file:")
                    .selected_text(&self.image_path)
                    .show_ui(ui, |ui| {
                        for file in &self.available_files {
                            ui.selectable_value(&mut self.image_path, file.clone(), file);
                        }
                    });
                if ui.button("Run Pipeline").clicked() {
                    self.run_pipeline();
                }
            });
            ui.separator();
            ui.label(format!("Status: {}", self.status));
        });

        egui::SidePanel::left("side_panel").resizable(false).show(ctx, |ui| {
            ui.heading("Flow Controls");
            ui.label("Add nodes and connect outputs to inputs.");
            ui.label("Drag the blocks around the canvas.");
            ui.separator();

            if ui.button("Add Load Image").clicked() {
                self.add_node(NodeKind::LoadImage);
            }
            if ui.button("Add Log Equalize").clicked() {
                self.add_node(NodeKind::LogEqualize);
            }
            if ui.button("Add Power Law Equalize").clicked() {
                self.add_node(NodeKind::PowerLawEqualize);
            }
            if ui.button("Add Display").clicked() {
                self.add_node(NodeKind::Display);
            }
            ui.separator();

            if let Some(port) = self.pending_connection {
                if let Some(node) = self.node_by_id(port.node_id) {
                    ui.label(format!("Pending: {} {:?} port", node.kind.title(), port.kind));
                }
                if ui.button("Cancel connection").clicked() {
                    self.pending_connection = None;
                }
            } else {
                ui.label("Click an output port first, then an input port.");
            }

            ui.separator();
            ui.heading("Connections");
            let mut remove_index = None;
            for (index, connection) in self.connections.iter().enumerate() {
                if let (Some(from), Some(to)) = (self.node_by_id(connection.from), self.node_by_id(connection.to)) {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} → {}", from.kind.title(), to.kind.title()));
                        if ui.small_button("x").clicked() {
                            remove_index = Some(index);
                        }
                    });
                }
            }
            if let Some(index) = remove_index {
                self.connections.remove(index);
            }
            ui.separator();
            ui.heading("Node Info");
            for node in &self.nodes {
                ui.label(format!("{}: {}", node.id, node.kind.title()));
                ui.label(node.kind.description());
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Block diagram canvas");
            let available_rect = ui.available_rect_before_wrap();
            ui.painter().rect_stroke(available_rect, 0.0, Stroke::new(1.0, Color32::LIGHT_GRAY));

            let selected_node = &mut self.selected_node;
            let pending_connection = &mut self.pending_connection;
            let connections = &mut self.connections;

            for node in &mut self.nodes {
                FlowApp::draw_node(ui, node, available_rect, selected_node, pending_connection, connections);
            }

            for connection in &self.connections {
                if let (Some(from), Some(to)) = (self.node_by_id(connection.from), self.node_by_id(connection.to)) {
                    let start = from.output_point();
                    let end = to.input_point();
                    let stroke = Stroke::new(3.0, Color32::from_rgb(120, 170, 255));
                    let dx = end.x - start.x;
                    let dy = end.y - start.y;
                    if dx.abs() > 50.0 && dy.abs() < 50.0 {
                        // Horizontal connection, bend it
                        let mid_x = (start.x + end.x) / 2.0;
                        let mid_y = start.y.min(end.y) - 40.0;
                        ui.painter().line_segment([start, Pos2::new(mid_x, mid_y)], stroke);
                        ui.painter().line_segment([Pos2::new(mid_x, mid_y), end], stroke);
                        // Arrowhead
                        let dir = (end - Pos2::new(mid_x, mid_y)).normalized();
                        let perp = Vec2::new(-dir.y, dir.x) * 5.0;
                        ui.painter().line_segment([end, end - dir * 10.0 + perp], stroke);
                        ui.painter().line_segment([end, end - dir * 10.0 - perp], stroke);
                    } else {
                        ui.painter().line_segment([start, end], stroke);
                        // Simple arrowhead for vertical
                        let dir = (end - start).normalized();
                        let perp = Vec2::new(-dir.y, dir.x) * 5.0;
                        ui.painter().line_segment([end, end - dir * 10.0 + perp], stroke);
                        ui.painter().line_segment([end, end - dir * 10.0 - perp], stroke);
                    }
                }
            }

            if let Some(start) = self.pending_connection {
                if let Some(pointer_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                    if let Some(node) = self.node_by_id(start.node_id) {
                        let start_pos = node.output_point();
                        let end = pointer_pos;
                        let stroke = Stroke::new(2.0, Color32::from_rgb(240, 220, 120));
                        let dx = end.x - start_pos.x;
                        let dy = end.y - start_pos.y;
                        if dx.abs() > 50.0 && dy.abs() < 50.0 {
                            let mid_x = (start_pos.x + end.x) / 2.0;
                            let mid_y = start_pos.y - 40.0;
                            ui.painter().line_segment([start_pos, Pos2::new(mid_x, mid_y)], stroke);
                            ui.painter().line_segment([Pos2::new(mid_x, mid_y), end], stroke);
                        } else {
                            ui.painter().line_segment([start_pos, end], stroke);
                        }
                    }
                }
            }
        });
    }
}

impl FlowApp {
    fn add_node(&mut self, kind: NodeKind) {
        let next_id = self.next_node_id;
        self.next_node_id += 1;
        let base_x = 80.0 + (self.nodes.len() as f32 % 4.0) * 260.0;
        let base_y = 220.0 + (self.nodes.len() as f32 / 4.0).floor() * 160.0;

        let params = match kind {
            NodeKind::LoadImage => NodeParams::None,
            NodeKind::LogEqualize => NodeParams::LogEqualize { c: 1.0 },
            NodeKind::PowerLawEqualize => NodeParams::PowerLawEqualize { c: 1.0, g: 0.5 },
            NodeKind::Display => NodeParams::None,
        };

        self.nodes.push(Node {
            id: next_id,
            kind,
            pos: Pos2::new(base_x, base_y),
            size: Vec2::new(220.0, 150.0),
            params,
        });
    }

    fn node_by_id(&self, node_id: usize) -> Option<&Node> {
        self.nodes.iter().find(|node| node.id == node_id)
    }

    fn draw_node(
        ui: &mut egui::Ui,
        node: &mut Node,
        canvas_rect: Rect,
        selected_node: &mut Option<usize>,
        pending_connection: &mut Option<PortRef>,
        connections: &mut Vec<Connection>,
    ) {
        let rect = node.rect();
        let response = ui.interact(rect, ui.id().with(node.id), Sense::click_and_drag());

        if response.clicked() {
            *selected_node = Some(node.id);
        }
        if response.dragged() {
            node.pos += response.drag_delta();
            // Clamp to canvas bounds
            node.pos.x = node.pos.x.clamp(canvas_rect.left(), canvas_rect.right() - node.size.x);
            node.pos.y = node.pos.y.clamp(canvas_rect.top(), canvas_rect.bottom() - node.size.y);
        }

        response.on_hover_text(node.kind.description());

        let background = if *selected_node == Some(node.id) {
            Color32::from_rgb(80, 120, 220)
        } else {
            Color32::from_rgb(70, 70, 80)
        };

        let painter = ui.painter();
        painter.rect_filled(rect, 10.0, background);
        painter.rect_stroke(rect, 10.0, Stroke::new(2.0, Color32::WHITE));

        painter.text(
            rect.center_top() + Vec2::new(0.0, 18.0),
            Align2::CENTER_TOP,
            node.kind.title(),
            egui::FontId::proportional(18.0),
            Color32::WHITE,
        );

        // Add parameter UI
        let center_x = rect.center().x;
        match &mut node.params {
            NodeParams::LogEqualize { c } => {
                let slider_rect = Rect::from_center_size(Pos2::new(center_x, node.pos.y + 90.0), Vec2::new(180.0, 20.0));
                ui.put(slider_rect, egui::Slider::new(c, 0.1..=10.0).text("c"));
            }
            NodeParams::PowerLawEqualize { c, g } => {
                let slider_rect1 = Rect::from_center_size(Pos2::new(center_x, node.pos.y + 80.0), Vec2::new(180.0, 20.0));
                ui.put(slider_rect1, egui::Slider::new(c, 0.1..=10.0).text("c"));
                let slider_rect2 = Rect::from_center_size(Pos2::new(center_x, node.pos.y + 105.0), Vec2::new(180.0, 20.0));
                ui.put(slider_rect2, egui::Slider::new(g, 0.1..=5.0).text("γ"));
            }
            _ => {}
        }

        // Now paint ports with a new painter borrow
        let painter = ui.painter();
        if node.kind.has_input() {
            let input_rect = Rect::from_center_size(node.input_point(), Vec2::splat(20.0));
            let input_resp = ui.interact(input_rect, ui.id().with((node.id, "in")), Sense::click());
            painter.circle_filled(node.input_point(), 8.0, Color32::WHITE);
            if input_resp.clicked() {
                if let Some(start) = *pending_connection {
                    if start.kind == PortKind::Output && start.node_id != node.id {
                        connections.push(Connection { from: start.node_id, to: node.id });
                        *pending_connection = None;
                    }
                }
            }
        }

        if node.kind.has_output() {
            let output_rect = Rect::from_center_size(node.output_point(), Vec2::splat(20.0));
            let output_resp = ui.interact(output_rect, ui.id().with((node.id, "out")), Sense::click());
            painter.circle_filled(node.output_point(), 8.0, Color32::WHITE);
            if output_resp.clicked() {
                *pending_connection = Some(PortRef { node_id: node.id, kind: PortKind::Output });
            }
        }
    }

    fn run_pipeline(&mut self) {
        self.status = "Processing pipeline...".to_owned();
        let path = self.build_pipeline_path();

        if path.is_empty() {
            self.status = "No valid pipeline found".to_owned();
            self.logs = "Connect a Load Image node to a Display node through processing nodes.".to_owned();
            return;
        }

        let cwd = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let full_path = cwd.join(&self.image_path);
        let image_path = full_path.to_str().unwrap_or(&self.image_path).to_owned();

        let mut data = match util::img2array(&image_path) {
            image => image,
        };
        let mut history = Vec::new();

        for node_id in path {
            if let Some(node) = self.node_by_id(node_id) {
                match node.kind {
                    NodeKind::LoadImage => history.push("Loaded image".to_owned()),
                    NodeKind::LogEqualize => {
                        if let NodeParams::LogEqualize { c } = node.params {
                            data = equalize::logeq(&data, c);
                            history.push(format!("Applied log equalization (c={:.2})", c));
                        }
                    }
                    NodeKind::PowerLawEqualize => {
                        if let NodeParams::PowerLawEqualize { c, g } = node.params {
                            data = equalize::powerlaweq(&data, c, g);
                            history.push(format!("Applied power-law equalization (c={:.2}, g={:.2})", c, g));
                        }
                    }
                    NodeKind::Display => {
                        history.push(format!(
                            "Output: {}x{} image, first cell {:.3}",
                            data.ncols(),
                            data.nrows(),
                            data[[0, 0]]
                        ));
                    }
                }
            }
        }

        self.status = "Pipeline complete".to_owned();
        self.logs = history.join("\n");
    }

    fn build_pipeline_path(&self) -> Vec<usize> {
        let start = self.nodes.iter().find(|node| node.kind == NodeKind::LoadImage);
        let Some(start) = start else { return Vec::new(); };

        let mut path = vec![start.id];
        let mut current = start.id;
        let mut visited = vec![current];

        while let Some(connection) = self.connections.iter().find(|conn| conn.from == current) {
            if visited.contains(&connection.to) {
                break;
            }
            visited.push(connection.to);
            path.push(connection.to);
            current = connection.to;
        }

        path
    }
}