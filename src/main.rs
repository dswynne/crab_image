// Standard
use std::env;

// External
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Sense, Stroke, Vec2};

// Local
mod equalize;
mod util;

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size(Vec2::new(900.0, 600.0)),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "crab_image Flow GUI",
        options,
        Box::new(|_cc| Ok(Box::new(FlowApp::default()))),
    ) {
        eprintln!("Failed to start GUI: {err}");
    }
}

struct FlowApp {
    nodes: Vec<Node>,
    connections: Vec<Connection>,
    pending_connection: Option<PortRef>,
    image_path: String,
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
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    LoadImage,
    LogEqualize,
    PowerLawEqualize,
    Display,
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

impl Default for FlowApp {
    fn default() -> Self {
        Self {
            nodes: vec![
                Node {
                    id: 1,
                    kind: NodeKind::LoadImage,
                    pos: Pos2::new(60.0, 80.0),
                    size: Vec2::new(220.0, 100.0),
                },
                Node {
                    id: 2,
                    kind: NodeKind::LogEqualize,
                    pos: Pos2::new(360.0, 80.0),
                    size: Vec2::new(220.0, 100.0),
                },
                Node {
                    id: 3,
                    kind: NodeKind::Display,
                    pos: Pos2::new(660.0, 80.0),
                    size: Vec2::new(220.0, 100.0),
                },
            ],
            connections: vec![Connection { from: 1, to: 2 }, Connection { from: 2, to: 3 }],
            pending_connection: None,
            image_path: "data/lena.tif".to_owned(),
            status: "Ready".to_owned(),
            logs: "Use the buttons below to add nodes, then connect them by clicking ports.".to_owned(),
            selected_node: None,
            next_node_id: 4,
        }
    }
}

impl eframe::App for FlowApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("crab_image Flow GUI");
                ui.separator();
                ui.label("Image file:");
                ui.text_edit_singleline(&mut self.image_path);
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
                FlowApp::draw_node(ui, node, selected_node, pending_connection, connections);
            }

            for connection in &self.connections {
                if let (Some(from), Some(to)) = (self.node_by_id(connection.from), self.node_by_id(connection.to)) {
                    ui.painter().line_segment(
                        [from.output_point(), to.input_point()],
                        Stroke::new(3.0, Color32::from_rgb(120, 170, 255)),
                    );
                }
            }

            if let Some(start) = self.pending_connection {
                if let Some(pointer_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                    if let Some(node) = self.node_by_id(start.node_id) {
                        let start_pos = node.output_point();
                        ui.painter().line_segment(
                            [start_pos, pointer_pos],
                            Stroke::new(2.0, Color32::from_rgb(240, 220, 120)),
                        );
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

        self.nodes.push(Node {
            id: next_id,
            kind,
            pos: Pos2::new(base_x, base_y),
            size: Vec2::new(220.0, 100.0),
        });
    }

    fn node_by_id(&self, node_id: usize) -> Option<&Node> {
        self.nodes.iter().find(|node| node.id == node_id)
    }

    fn draw_node(
        ui: &mut egui::Ui,
        node: &mut Node,
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
        }

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
        painter.text(
            rect.center() + Vec2::new(0.0, 6.0),
            Align2::CENTER_CENTER,
            node.kind.description(),
            egui::FontId::proportional(12.0),
            Color32::LIGHT_GRAY,
        );

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
                        data = equalize::logeq(&data, 1.0);
                        history.push("Applied log equalization".to_owned());
                    }
                    NodeKind::PowerLawEqualize => {
                        data = equalize::powerlaweq(&data, 1.0, 0.5);
                        history.push("Applied power-law equalization".to_owned());
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


