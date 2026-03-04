const API_BASE = "/api";

// ── Workflow Templates ────────────────────────────────────────────────────

const WORKFLOW_TEMPLATES = {
  "nextflow-rnaseq": {
    workflow_url: "https://github.com/nf-core/rnaseq",
    workflow_type: "NFL",
    workflow_type_version: "DSL2",
    workflow_engine: "nextflow",
    workflow_engine_version: "24.10.0",
    workflow_params: { input: "samplesheet.csv", outdir: "./results", genome: "GRCh38" },
    tags: { project: "rnaseq-analysis", pipeline: "nf-core/rnaseq" },
  },
  "cwl-variant": {
    workflow_url: "https://github.com/example/variant-calling.cwl",
    workflow_type: "CWL",
    workflow_type_version: "v1.2",
    workflow_engine: "cwltool",
    workflow_engine_version: "3.1.20240112164112",
    workflow_params: { input_file: "s3://bucket/input.vcf", reference: "hg38.fa" },
    tags: { project: "genomics", workflow: "variant-calling" },
  },
  "snakemake-demo": {
    workflow_url: "https://github.com/example/snakemake-demo",
    workflow_type: "SMK",
    workflow_type_version: "7.32",
    workflow_engine: "snakemake",
    workflow_engine_version: "7.32.4",
    workflow_params: { samples: "samples.txt", output: "results/" },
    tags: { project: "demo", workflow: "snakemake-demo" },
  },
};

// ── JSON editor helpers ───────────────────────────────────────────────────

function escHtml(s) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function tokenize(json) {
  const tokens = [];
  const re =
    /("(?:[^\\"]|\\.)*")(?=\s*:)|("(?:[^\\"]|\\.)*")|(true|false|null)|(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)|([{}[\],:])/g;
  let last = 0;
  let m = re.exec(json);
  while (m !== null) {
    if (m.index > last) tokens.push(escHtml(json.slice(last, m.index)));
    if (m[1] !== undefined) tokens.push(`<span class="json-key">${escHtml(m[1])}</span>`);
    else if (m[2] !== undefined) tokens.push(`<span class="json-str">${escHtml(m[2])}</span>`);
    else if (m[3] !== undefined) tokens.push(`<span class="json-bool">${escHtml(m[3])}</span>`);
    else if (m[4] !== undefined) tokens.push(`<span class="json-num">${escHtml(m[4])}</span>`);
    else if (m[5] !== undefined) tokens.push(`<span class="json-punct">${escHtml(m[5])}</span>`);
    last = re.lastIndex;
    m = re.exec(json);
  }
  if (last < json.length) tokens.push(escHtml(json.slice(last)));
  return tokens.join("");
}

function getEditorText(id) {
  return document.getElementById(id).innerText;
}

function setEditorHtml(id, html) {
  document.getElementById(id).innerHTML = html;
}

function highlightEditor(id) {
  const raw = getEditorText(id).trim();
  try {
    const pretty = JSON.stringify(JSON.parse(raw || "{}"), null, 2);
    setEditorHtml(id, tokenize(pretty));
    clearEditorError(id);
  } catch {
    // invalid JSON — leave as-is
  }
}

function _formatEditor(id) {
  const raw = getEditorText(id).trim() || "{}";
  try {
    setEditorHtml(id, tokenize(JSON.stringify(JSON.parse(raw), null, 2)));
    clearEditorError(id);
  } catch {
    showEditorError(id, "Invalid JSON");
  }
}

function clearEditor(id) {
  setEditorHtml(id, tokenize("{\n\n}"));
  clearEditorError(id);
}

function parseEditor(id) {
  const raw = getEditorText(id).trim();
  if (!raw || raw === "{}" || raw === "{\n\n}") return null;
  const val = JSON.parse(raw);
  return Object.keys(val).length === 0 ? null : val;
}

function showEditorError(id, msg) {
  document.getElementById(`${id}_err`).textContent = msg;
  document.getElementById(`${id}-wrap`).classList.add("is-invalid");
}

function clearEditorError(id) {
  document.getElementById(`${id}_err`).textContent = "";
  document.getElementById(`${id}-wrap`).classList.remove("is-invalid");
}

// Attach blur highlight + Tab→spaces for each editor
["wp", "tags", "ep"].forEach((id) => {
  const el = document.getElementById(id);
  el.addEventListener("blur", () => highlightEditor(id));
  el.addEventListener("keydown", (e) => {
    if (e.key === "Tab") {
      e.preventDefault();
      document.execCommand("insertText", false, "  ");
    }
  });
});

// ── Templates ─────────────────────────────────────────────────────────────

function _loadTemplate(key) {
  const t = WORKFLOW_TEMPLATES[key];
  if (!t) return;
  document.getElementById("workflow_url").value = t.workflow_url;
  document.getElementById("workflow_type").value = t.workflow_type;
  document.getElementById("workflow_type_version").value = t.workflow_type_version;
  document.getElementById("workflow_engine").value = t.workflow_engine || "";
  document.getElementById("workflow_engine_version").value = t.workflow_engine_version || "";
  if (t.workflow_params) setEditorHtml("wp", tokenize(JSON.stringify(t.workflow_params, null, 2)));
  if (t.tags) setEditorHtml("tags", tokenize(JSON.stringify(t.tags, null, 2)));
  if (t.workflow_engine_parameters)
    setEditorHtml("ep", tokenize(JSON.stringify(t.workflow_engine_parameters, null, 2)));
}

// ── Form field validation ─────────────────────────────────────────────────

function getField(id) {
  return document.getElementById(id).value.trim();
}

function setInvalid(id, msg) {
  document.getElementById(id).classList.add("is-invalid");
  const err = document.getElementById(`${id}_err`);
  if (err) err.textContent = msg;
}

function clearInvalid(id) {
  document.getElementById(id).classList.remove("is-invalid");
  const err = document.getElementById(`${id}_err`);
  if (err) err.textContent = "";
}

// ── Form actions ──────────────────────────────────────────────────────────

function showFormError(msg) {
  const el = document.getElementById("form-error");
  el.textContent = msg;
  el.classList.add("visible");
}

function clearFormError() {
  const el = document.getElementById("form-error");
  el.textContent = "";
  el.classList.remove("visible");
}

function setSubmitting(loading) {
  const btn = document.getElementById("submit-btn");
  const label = document.getElementById("submit-label");
  const spinner = document.getElementById("submit-spinner");
  btn.disabled = loading;
  spinner.hidden = !loading;
  label.textContent = loading ? "Submitting…" : "Submit Run";
}

async function _submitRun(e) {
  e.preventDefault();
  clearFormError();

  const required = ["workflow_url", "workflow_type", "workflow_type_version"];
  let valid = true;

  required.forEach((id) => {
    clearInvalid(id);
    if (!getField(id)) {
      setInvalid(id, "This field is required");
      valid = false;
    }
  });

  const editorMap = [
    { id: "wp", key: "workflow_params" },
    { id: "tags", key: "tags" },
    { id: "ep", key: "workflow_engine_parameters" },
  ];

  const jsonVals = {};
  editorMap.forEach(({ id, key }) => {
    clearEditorError(id);
    try {
      jsonVals[key] = parseEditor(id);
    } catch {
      showEditorError(id, "Invalid JSON — check syntax");
      valid = false;
    }
  });

  if (!valid) {
    showFormError("Please fix the errors above before submitting.");
    return;
  }

  const body = {
    workflow_url: getField("workflow_url"),
    workflow_type: getField("workflow_type"),
    workflow_type_version: getField("workflow_type_version"),
  };
  const engine = getField("workflow_engine");
  const engineVer = getField("workflow_engine_version");
  if (engine) body.workflow_engine = engine;
  if (engineVer) body.workflow_engine_version = engineVer;
  if (jsonVals.workflow_params) body.workflow_params = jsonVals.workflow_params;
  if (jsonVals.tags) body.tags = jsonVals.tags;
  if (jsonVals.workflow_engine_parameters)
    body.workflow_engine_parameters = jsonVals.workflow_engine_parameters;

  setSubmitting(true);
  try {
    const res = await fetch(`${API_BASE}/runs`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      throw new Error(err.msg || `Server returned ${res.status}`);
    }
    const data = await res.json();
    showSuccess(data.run_id);
  } catch (err) {
    showFormError(`Submission failed: ${err.message}`);
  } finally {
    setSubmitting(false);
  }
}

function showSuccess(runId) {
  const safe = runId.replace(/[^a-zA-Z0-9_-]/g, "");
  const wrap = document.getElementById("success-banner-wrap");
  wrap.innerHTML = `
    <div class="success-banner">
      <span>Run submitted — ID: <code>${safe}</code></span>
      <a href="/run.html?run_id=${encodeURIComponent(runId)}">View run →</a>
    </div>`;
  wrap.scrollIntoView({ behavior: "smooth", block: "nearest" });
}

function _resetForm() {
  document.getElementById("submit-form").reset();
  ["workflow_url", "workflow_type", "workflow_type_version"].forEach(clearInvalid);
  ["wp", "tags", "ep"].forEach((id) => {
    clearEditor(id);
    clearEditorError(id);
  });
  clearFormError();
  document.getElementById("success-banner-wrap").innerHTML = "";
}
