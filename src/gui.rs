// Standard
use std::{env, fs, net::SocketAddr, path::PathBuf};

// External
use axum::{
    extract::{Json, Path},
    http::{header, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use axum_server::Server;
use serde::{Deserialize, Serialize};

// Local
use crate::equalize;
use crate::util;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    LoadImage,
    LogEqualize,
    PowerLawEqualize,
    Display,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum NodeParams {
    None,
    LogEqualize { c: f32 },
    PowerLawEqualize { c: f32, g: f32 },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Node {
    pub id: usize,
    pub kind: NodeKind,
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub params: NodeParams,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Connection {
    pub from: usize,
    pub to: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PipelineData {
    pub nodes: Vec<Node>,
    pub connections: Vec<Connection>,
    pub image_path: String,
}

#[derive(Deserialize)]
struct SavePipelineRequest {
    filename: String,
    pipeline: PipelineData,
}

#[derive(Serialize)]
struct SavePipelineResponse {
    name: String,
}

#[derive(Serialize)]
struct RunPipelineResponse {
    logs: String,
}

pub async fn start_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = Router::new()
        .route("/api/files", get(list_files))
        .route("/api/pipelines", get(list_pipelines).post(save_pipeline))
        .route("/api/pipelines/:name", get(load_pipeline))
        .route("/api/run", post(run_pipeline))
        .route("/", get(serve_index))
        .route("/index.html", get(serve_index))
        .route("/main.js", get(serve_main_js))
        .route("/data/:file", get(serve_data_file));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    Server::bind(addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn serve_index() -> impl IntoResponse {
    match tokio::fs::read("web/index.html").await {
        Ok(bytes) => Html(bytes).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Unable to load index").into_response(),
    }
}

async fn serve_main_js() -> impl IntoResponse {
    match tokio::fs::read("web/main.js").await {
        Ok(bytes) => (
            [(header::CONTENT_TYPE, "application/javascript")],
            bytes,
        )
            .into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Unable to load main.js").into_response(),
    }
}

async fn serve_data_file(Path(file): Path<String>) -> impl IntoResponse {
    if file.contains("..") {
        return (StatusCode::BAD_REQUEST, "Invalid file name").into_response();
    }

    let path = PathBuf::from("data").join(&file);
    if !path.exists() {
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    }

    let content_type = match path.extension().and_then(|s| s.to_str()) {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("tif") | Some("tiff") => "image/tiff",
        _ => "application/octet-stream",
    };

    match tokio::fs::read(&path).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, content_type)], bytes).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Unable to load file").into_response(),
    }
}

async fn list_files() -> Result<Json<Vec<String>>, (StatusCode, String)> {
    match collect_image_files() {
        Ok(files) => Ok(Json(files)),
        Err(err) => Err((StatusCode::INTERNAL_SERVER_ERROR, err)),
    }
}

async fn list_pipelines() -> Result<Json<Vec<String>>, (StatusCode, String)> {
    match collect_pipeline_names() {
        Ok(pipelines) => Ok(Json(pipelines)),
        Err(err) => Err((StatusCode::INTERNAL_SERVER_ERROR, err)),
    }
}

async fn load_pipeline(
    Path(name): Path<String>,
) -> Result<Json<PipelineData>, (StatusCode, String)> {
    let sanitized_name = sanitize_filename(&name)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Invalid pipeline name".to_string()))?;
    let filepath = PathBuf::from("pipelines").join(&sanitized_name);

    let yaml = fs::read_to_string(&filepath)
        .map_err(|err| (StatusCode::NOT_FOUND, format!("Failed to read pipeline: {err}")))?;
    let pipeline = serde_yaml::from_str(&yaml)
        .map_err(|err| (StatusCode::BAD_REQUEST, format!("Malformed pipeline: {err}")))?;
    Ok(Json(pipeline))
}

async fn save_pipeline(
    Json(payload): Json<SavePipelineRequest>,
) -> Result<Json<SavePipelineResponse>, (StatusCode, String)> {
    let sanitized_name = sanitize_filename(&payload.filename)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Invalid filename".to_string()))?;

    let filename = if sanitized_name.ends_with(".yaml") || sanitized_name.ends_with(".yml") {
        sanitized_name
    } else {
        format!("{sanitized_name}.yaml")
    };

    let filepath = PathBuf::from("pipelines").join(&filename);
    fs::create_dir_all("pipelines")
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, format!("Unable to create pipelines directory: {err}")))?;

    let yaml = serde_yaml::to_string(&payload.pipeline)
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, format!("Serialization failed: {err}")))?;
    fs::write(&filepath, yaml)
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save pipeline: {err}")))?;

    Ok(Json(SavePipelineResponse { name: filename }))
}

async fn run_pipeline(
    Json(pipeline): Json<PipelineData>,
) -> Result<Json<RunPipelineResponse>, (StatusCode, String)> {
    let logs = execute_pipeline(&pipeline)
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err))?;
    Ok(Json(RunPipelineResponse { logs }))
}

fn sanitize_filename(filename: &str) -> Option<String> {
    let path = PathBuf::from(filename);
    let file_name = path.file_name()?.to_str()?;
    if file_name.contains('/') || file_name.contains('\\') {
        return None;
    }
    if file_name.starts_with('.') {
        return None;
    }
    Some(file_name.to_string())
}

fn collect_image_files() -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    let directory = PathBuf::from("data");
    if directory.exists() {
        for entry in fs::read_dir(&directory).map_err(|err| format!("Failed to read data directory: {err}"))? {
            let entry = entry.map_err(|err| format!("Failed to read data entry: {err}"))?;
            let path = entry.path();
            if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
                if fname.ends_with(".tif") || fname.ends_with(".png") || fname.ends_with(".jpg") || fname.ends_with(".jpeg") {
                    files.push(format!("data/{fname}"));
                }
            }
        }
    }
    files.sort();
    Ok(files)
}

fn collect_pipeline_names() -> Result<Vec<String>, String> {
    let mut pipelines = Vec::new();
    let directory = PathBuf::from("pipelines");
    if directory.exists() {
        for entry in fs::read_dir(&directory).map_err(|err| format!("Failed to read pipelines directory: {err}"))? {
            let entry = entry.map_err(|err| format!("Failed to read pipeline entry: {err}"))?;
            let path = entry.path();
            if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
                if fname.ends_with(".yaml") || fname.ends_with(".yml") {
                    pipelines.push(fname.to_string());
                }
            }
        }
    }
    pipelines.sort();
    Ok(pipelines)
}

fn execute_pipeline(pipeline: &PipelineData) -> Result<String, String> {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let full_path = cwd.join(&pipeline.image_path);
    let image_path = full_path
        .to_str()
        .ok_or_else(|| "Invalid image path".to_string())?
        .to_owned();

    if !PathBuf::from(&image_path).exists() {
        return Err(format!("Image file not found: {image_path}"));
    }

    let mut image_data = util::img2array(&image_path);
    let mut history = Vec::new();

    let start = pipeline
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::LoadImage)
        .ok_or_else(|| "No Load Image node found".to_string())?;

    let mut path = vec![start.id];
    let mut current = start.id;
    let mut visited = vec![current];

    while let Some(connection) = pipeline.connections.iter().find(|conn| conn.from == current) {
        if visited.contains(&connection.to) {
            break;
        }
        visited.push(connection.to);
        path.push(connection.to);
        current = connection.to;
    }

    for node_id in path {
        if let Some(node) = pipeline.nodes.iter().find(|n| n.id == node_id) {
            match node.kind {
                NodeKind::LoadImage => history.push("Loaded image".to_string()),
                NodeKind::LogEqualize => {
                    if let NodeParams::LogEqualize { c } = node.params {
                        image_data = equalize::logeq(&image_data, c);
                        history.push(format!("Applied log equalization (c={:.2})", c));
                    }
                }
                NodeKind::PowerLawEqualize => {
                    if let NodeParams::PowerLawEqualize { c, g } = node.params {
                        image_data = equalize::powerlaweq(&image_data, c, g);
                        history.push(format!("Applied power-law equalization (c={:.2}, g={:.2})", c, g));
                    }
                }
                NodeKind::Display => {
                    history.push(format!(
                        "Output: {}x{} image, first cell {:.3}",
                        image_data.ncols(),
                        image_data.nrows(),
                        image_data[[0, 0]]
                    ));
                }
            }
        }
    }

    util::array2img(&image_data, "data/output.png");
    history.push("Saved output image to data/output.png".to_string());
    Ok(history.join("\n"))
}

pub fn run_pipeline_cli(filepath: &str) -> Result<String, String> {
    let yaml = fs::read_to_string(filepath)
        .map_err(|e| format!("Failed to read file: {e}"))?;
    let data: PipelineData = serde_yaml::from_str(&yaml)
        .map_err(|e| format!("Deserialization error: {e}"))?;

    execute_pipeline(&data)
}
