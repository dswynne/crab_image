const api = {
  files: "/api/files",
  pipelines: "/api/pipelines",
  run: "/api/run",
  pipeline: (name) => `/api/pipelines/${encodeURIComponent(name)}`,
};

const graph = new LiteGraph.LGraph();
let canvas;

function registerNodeTypes() {
  function register(type, title, inputs, outputs, properties = []) {
    function Node() {
      LiteGraph.LGraphNode.call(this);
      inputs.forEach(([name, slotType]) => this.addInput(name, slotType));
      outputs.forEach(([name, slotType]) => this.addOutput(name, slotType));
      properties.forEach(({ name, defaultValue }) => this.addProperty(name, defaultValue));
    }

    Node.title = title;
    Node.prototype.getTitle = function () {
      return title;
    };

    LiteGraph.extend(Node, LiteGraph.LGraphNode);
    LiteGraph.registerNodeType(`crab/${type}`, Node);
  }

  register("LoadImage", "Load Image", [], [["image", "image"]]);
  register("LogEqualize", "Log Equalize", [["image", "image"]], [["image", "image"]], [
    { name: "c", defaultValue: 1.0 },
  ]);
  register("PowerLawEqualize", "Power Law Equalize", [["image", "image"]], [["image", "image"]], [
    { name: "c", defaultValue: 1.0 },
    { name: "g", defaultValue: 0.5 },
  ]);
  register("Display", "Display Result", [["image", "image"]], []);
}

function createNode(type, x = 40, y = 40) {
  const node = LiteGraph.createNode(`crab/${type}`);
  node.pos = [x, y];
  graph.add(node);
  updateGraphInfo();
  return node;
}

function updateGraphInfo() {
  const info = document.getElementById("graphInfo");
  const nodes = graph._nodes ? graph._nodes.length : 0;
  const links = graph.links ? graph.links.length : 0;
  info.textContent = `${nodes} nodes · ${links} links`;
}

function updateStatus(message) {
  document.getElementById("statusMessage").textContent = message;
}

function updateLog(message) {
  document.getElementById("logText").textContent = message;
}

function refreshOutputImage() {
  const image = document.getElementById("outputImage");
  image.src = `/data/output.png?ts=${Date.now()}`;
}

async function fetchJson(url, options = {}) {
  const response = await fetch(url, options);
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || response.statusText);
  }
  return response.json();
}

function pipelineParamsForNode(node) {
  if (node.type.endsWith("/LoadImage") || node.type.endsWith("/Display")) {
    return "None";
  }

  if (node.type.endsWith("/LogEqualize")) {
    return { LogEqualize: { c: node.properties.c ?? 1.0 } };
  }

  if (node.type.endsWith("/PowerLawEqualize")) {
    return {
      PowerLawEqualize: {
        c: node.properties.c ?? 1.0,
        g: node.properties.g ?? 0.5,
      },
    };
  }

  return "None";
}

function serializePipeline() {
  const serialized = {
    nodes: [],
    connections: [],
    image_path: document.getElementById("imageSelector").value,
  };

  graph._nodes.forEach((node) => {
    serialized.nodes.push({
      id: node.id,
      kind: node.type.replace(/^crab\//, ""),
      pos: [node.pos[0], node.pos[1]],
      size: [node.size[0], node.size[1]],
      params: pipelineParamsForNode(node),
    });
  });

  graph.links.forEach((link) => {
    serialized.connections.push({
      from: link.origin_id,
      to: link.target_id,
    });
  });

  return serialized;
}

function deserializePipeline(pipeline) {
  graph.clear();

  const maxId = pipeline.nodes.reduce((max, node) => Math.max(max, node.id), 0);
  pipeline.nodes.forEach((nodeData) => {
    const node = LiteGraph.createNode(`crab/${nodeData.kind}`);
    node.id = nodeData.id;
    node.pos = [nodeData.pos[0], nodeData.pos[1]];
    node.size = [nodeData.size[0], nodeData.size[1]];
    graph.add(node);

    if (typeof nodeData.params === 'object') {
      if (nodeData.params.LogEqualize) {
        node.properties.c = nodeData.params.LogEqualize.c;
      }
      if (nodeData.params.PowerLawEqualize) {
        node.properties.c = nodeData.params.PowerLawEqualize.c;
        node.properties.g = nodeData.params.PowerLawEqualize.g;
      }
    }
  });

  graph.last_id = maxId + 1;

  pipeline.connections.forEach((conn) => {
    graph.connect(conn.from, 0, conn.to, 0);
  });

  updateGraphInfo();
  canvas.draw(true, true);
}

async function refreshAvailableFiles() {
  try {
    const files = await fetchJson(api.files);
    const selector = document.getElementById("imageSelector");
    selector.innerHTML = "";
    files.forEach((file) => {
      const option = document.createElement("option");
      option.value = file;
      option.textContent = file.replace(/^data\//, "");
      selector.appendChild(option);
    });
  } catch (error) {
    updateLog(`Unable to load image files: ${error.message}`);
  }
}

async function refreshPipelineList() {
  try {
    const pipelines = await fetchJson(api.pipelines);
    const selector = document.getElementById("pipelineSelector");
    selector.innerHTML = "";
    pipelines.forEach((pipeline) => {
      const option = document.createElement("option");
      option.value = pipeline;
      option.textContent = pipeline;
      selector.appendChild(option);
    });
  } catch (error) {
    updateLog(`Unable to load pipelines: ${error.message}`);
  }
}

async function newGraph() {
  graph.clear();
  const load = createNode("LoadImage", 40, 160);
  const log = createNode("LogEqualize", 340, 160);
  const display = createNode("Display", 640, 160);
  graph.connect(load.id, 0, log.id, 0);
  graph.connect(log.id, 0, display.id, 0);
  updateGraphInfo();
  canvas.draw(true, true);
}

async function savePipeline() {
  const name = window.prompt("Enter filename for the pipeline", "pipeline");
  if (!name) {
    return;
  }

  const payload = {
    filename: name,
    pipeline: serializePipeline(),
  };

  try {
    const result = await fetchJson(api.pipelines, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    updateStatus(`Saved ${result.name}`);
    updateLog(`Pipeline saved: ${result.name}`);
    await refreshPipelineList();
  } catch (error) {
    updateLog(`Save failed: ${error.message}`);
  }
}

async function loadPipeline() {
  const selector = document.getElementById("pipelineSelector");
  const name = selector.value;
  if (!name) {
    return;
  }

  try {
    const pipeline = await fetchJson(api.pipeline(name));
    deserializePipeline(pipeline);
    if (pipeline.image_path) {
      document.getElementById("imageSelector").value = pipeline.image_path;
    }
    updateStatus(`Loaded ${name}`);
    updateLog(`Pipeline loaded from ${name}`);
  } catch (error) {
    updateLog(`Load failed: ${error.message}`);
  }
}

async function runPipeline() {
  const payload = serializePipeline();
  updateStatus("Running pipeline...");

  try {
    const result = await fetchJson(api.run, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    updateStatus("Pipeline complete");
    updateLog(result.logs);
    refreshOutputImage();
  } catch (error) {
    updateStatus("Pipeline failed");
    updateLog(`Error: ${error.message}`);
  }
}

// Initialize the application
async function init() {
  registerNodeTypes();
  graph.start();

  canvas = new LiteGraph.LGraphCanvas("#graphcanvas", graph);
  canvas.draw(true, true);

  // Load initial data
  await refreshAvailableFiles();
  await refreshPipelineList();

  // Attach event listeners
  document.getElementById("runPipelineButton").addEventListener("click", runPipeline);
  document.getElementById("savePipelineButton").addEventListener("click", savePipeline);
  document.getElementById("loadPipelineButton").addEventListener("click", loadPipeline);

  document.getElementById("addLoadImage").addEventListener("click", () => createNode("LoadImage"));
  document.getElementById("addLogEqualize").addEventListener("click", () => createNode("LogEqualize"));
  document.getElementById("addPowerLawEqualize").addEventListener("click", () => createNode("PowerLawEqualize"));
  document.getElementById("addDisplay").addEventListener("click", () => createNode("Display"));

  document.getElementById("newGraph").addEventListener("click", newGraph);

  updateStatus("Ready");
  updateLog("Drag nodes and connect outputs to inputs.");
}

// Start the app when the page loads
window.addEventListener("load", init);

window.addEventListener("load", async () => {
  registerNodeTypes();
  canvas = new LiteGraph.LGraphCanvas("#graphcanvas", graph);
  canvas.background_image = null;
  canvas.clear();

  document.getElementById("addLoadImage").addEventListener("click", () => createNode("LoadImage", 40, 80));
  document.getElementById("addLogEqualize").addEventListener("click", () => createNode("LogEqualize", 40, 80));
  document.getElementById("addPowerLawEqualize").addEventListener("click", () => createNode("PowerLawEqualize", 40, 80));
  document.getElementById("addDisplay").addEventListener("click", () => createNode("Display", 40, 80));
  document.getElementById("runPipelineButton").addEventListener("click", runPipeline);
  document.getElementById("savePipelineButton").addEventListener("click", savePipeline);
  document.getElementById("loadPipelineButton").addEventListener("click", loadPipeline);
  document.getElementById("newGraph").addEventListener("click", newGraph);

  await refreshAvailableFiles();
  await refreshPipelineList();
  await newGraph();

  setInterval(updateGraphInfo, 500);
});
