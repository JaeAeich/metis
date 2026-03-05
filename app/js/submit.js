const API_BASE = "/api";

// ── Service-info state ─────────────────────────────────────────────────────

let registeredEngines = []; // EngineInfo[]

async function loadServiceInfo() {
  try {
    const [infoRes] = await Promise.all([fetch(`${API_BASE}/service-info`), loadTemplates()]);
    if (!infoRes.ok) throw new Error(`status ${infoRes.status}`);
    const info = await infoRes.json();
    registeredEngines = info.engines || [];
  } catch {
    registeredEngines = [];
  }
  populateEngineSelect();
  populateTemplates();
  if (registeredEngines.length === 0) {
    document.getElementById("no-engines-banner").hidden = false;
    document.getElementById("submit-btn").disabled = true;
    populateWorkflowTypes(null);
    populateWorkflowTypeVersions(null);
  }
}

// ── Engine select ──────────────────────────────────────────────────────────

function populateEngineSelect() {
  const sel = document.getElementById("workflow_engine_select");
  sel.innerHTML = "";

  if (registeredEngines.length === 0) {
    sel.innerHTML = '<option value="">No engines</option>';
    return;
  }

  if (registeredEngines.length > 1) {
    const placeholder = document.createElement("option");
    placeholder.value = "";
    placeholder.textContent = "Select…";
    sel.appendChild(placeholder);
  }

  registeredEngines.forEach((eng) => {
    const opt = document.createElement("option");
    opt.value = eng.name;
    opt.textContent = `${eng.name} (${eng.version})`;
    sel.appendChild(opt);
  });

  if (registeredEngines.length === 1) {
    sel.value = registeredEngines[0].name;
    onEngineChange();
  } else {
    populateWorkflowTypes(null);
    populateWorkflowTypeVersions(null);
  }
}

function onEngineChange() {
  const sel = document.getElementById("workflow_engine_select");
  const name = sel.value;
  const eng = registeredEngines.find((e) => e.name === name) || null;

  document.getElementById("workflow_engine").value = eng ? eng.name : "";
  document.getElementById("workflow_engine_version").value = eng ? eng.version : "";

  populateWorkflowTypes(eng);
  populateWorkflowTypeVersions(eng);
  renderEngineInfoTooltip(eng);

  const tooltip = document.getElementById("engine-info-tooltip");
  tooltip.hidden = !eng;

  // Refresh template pills for the newly selected engine
  if (eng) {
    const cardsEl = document.getElementById("template-cards");
    renderTemplateCards(eng, cardsEl);
  }
}

// ── Workflow type select ───────────────────────────────────────────────────

const STATIC_TYPES = [
  { value: "NFL", label: "NFL (Nextflow)" },
  { value: "CWL", label: "CWL" },
  { value: "WDL", label: "WDL" },
  { value: "SMK", label: "SMK (Snakemake)" },
];

function populateWorkflowTypes(eng) {
  const sel = document.getElementById("workflow_type");
  const current = sel.value;
  sel.innerHTML = '<option value="">Select…</option>';

  const types =
    eng?.workflow_types && eng.workflow_types.length > 0
      ? eng.workflow_types
      : STATIC_TYPES.map((t) => t.value);

  types.forEach((t) => {
    const opt = document.createElement("option");
    opt.value = t;
    opt.textContent = STATIC_TYPES.find((s) => s.value === t)?.label ?? t;
    sel.appendChild(opt);
  });

  if (current && types.includes(current)) sel.value = current;
  if (types.length === 1) sel.value = types[0];
}

// ── Workflow type version select ───────────────────────────────────────────

function populateWorkflowTypeVersions(eng) {
  const sel = document.getElementById("workflow_type_version");
  const current = sel.value;
  sel.innerHTML = '<option value="">Select…</option>';

  const versions =
    eng?.workflow_type_versions && eng.workflow_type_versions.length > 0
      ? eng.workflow_type_versions
      : [];

  versions.forEach((v) => {
    const opt = document.createElement("option");
    opt.value = v;
    opt.textContent = v;
    sel.appendChild(opt);
  });

  if (current && versions.includes(current)) sel.value = current;
  if (versions.length === 1) sel.value = versions[0];
}

// ── Engine info tooltip ────────────────────────────────────────────────────

function renderEngineInfoTooltip(eng) {
  if (!eng) return;

  document.getElementById("engine-info-popup-title").textContent = `${eng.name} ${eng.version}`;

  const body = document.getElementById("engine-info-popup-body");
  const params = eng.engine_params?.validatedParams ?? [];

  if (params.length === 0) {
    body.innerHTML = '<p class="eip-empty">No params defined.</p>';
    return;
  }

  body.innerHTML = params
    .map((p) => {
      const names = (p.names || []).join(", ");
      const type = p.type ?? "string";

      const badges = [];
      if (p.required) badges.push('<span class="eip-badge eip-badge--req">req</span>');
      if (p.sensitive) badges.push('<span class="eip-badge eip-badge--sen">sensitive</span>');

      let extra = "";

      if (p.validate) {
        for (const v of p.validate) {
          if (v.type === "enum" && v.allowed) {
            extra += `<span class="eip-allowed">${v.allowed.map(escHtml).join(" · ")}</span>`;
          }
          if (v.type === "range" && (v.min != null || v.max != null)) {
            const parts = [
              v.min != null ? `≥${v.min}` : null,
              v.max != null ? `≤${v.max}` : null,
            ].filter(Boolean);
            extra += `<span class="eip-range">${parts.join(", ")}</span>`;
          }
        }
      }

      if (p.default != null && !p.sensitive) {
        const def = typeof p.default === "object" ? JSON.stringify(p.default) : String(p.default);
        extra += `<span class="eip-default">default: ${escHtml(def)}</span>`;
      }

      const desc = p.description ? `<div class="eip-desc">${escHtml(p.description)}</div>` : "";

      return `<div class="eip-row">
  <div class="eip-row-top">
    <code class="eip-name">${escHtml(names)}</code>
    <span class="eip-type">${escHtml(type)}</span>
    ${badges.join("")}
  </div>
  ${extra || desc ? `<div class="eip-row-meta">${extra}${desc}</div>` : ""}
</div>`;
    })
    .join("");
}

// ── Templates ──────────────────────────────────────────────────────────────

// Templates loaded from /public/templates.json (edit that file to add/remove templates)
let ENGINE_TEMPLATES = {};

async function loadTemplates() {
  try {
    const res = await fetch("/public/templates.json");
    if (res.ok) ENGINE_TEMPLATES = await res.json();
  } catch {
    // silently ignore — templates just won't appear
  }
}

function populateTemplates() {
  if (registeredEngines.length === 0) {
    document.getElementById("template-area").hidden = true;
    return;
  }

  document.getElementById("template-area").hidden = false;

  // Render templates for the currently selected engine (or first engine)
  const sel = document.getElementById("workflow_engine_select");
  const engName = sel.value || (registeredEngines[0]?.name ?? "");
  const eng = registeredEngines.find((e) => e.name === engName) || registeredEngines[0];
  if (eng) renderTemplateCards(eng, document.getElementById("template-cards"));
}

function renderTemplateCards(eng, container) {
  const allTemplates = ENGINE_TEMPLATES[eng.name.toLowerCase()] || [];
  const supportedTypes = eng.workflow_types || [];
  const supportedVersions = eng.workflow_type_versions || [];

  const templates = allTemplates.filter((tpl) => {
    const typeMatch = supportedTypes.length === 0 || supportedTypes.includes(tpl.workflow_type);
    const versionMatch =
      supportedVersions.length === 0 || supportedVersions.includes(tpl.workflow_type_version);
    return typeMatch && versionMatch;
  });

  container.innerHTML = "";

  if (templates.length === 0) {
    container.innerHTML = '<span class="tpl-empty">No templates.</span>';
    return;
  }

  templates.forEach((tpl, i) => {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = `tpl-pill${i === 0 ? " is-active" : ""}`;
    btn.title = tpl.description || "";
    btn.textContent = tpl.label;
    btn.addEventListener("click", () => {
      container.querySelectorAll(".tpl-pill").forEach((b) => {
        b.classList.remove("is-active");
      });
      btn.classList.add("is-active");
      applyTemplateFields(tpl);
    });
    container.appendChild(btn);
  });

  // Auto-fill fields from first template (engine already selected)
  applyTemplateFields(templates[0]);
}

function _applyTemplate(eng, _tpl) {
  // Select engine in chip and sync hidden inputs
  const engSel = document.getElementById("workflow_engine_select");
  engSel.value = eng.name;
  onEngineChange();

  renderTemplateCards(eng, document.getElementById("template-cards"));
}

function applyTemplateFields(tpl) {
  document.getElementById("workflow_url").value = tpl.workflow_url;

  // Set type + version after onEngineChange populates selects
  document.getElementById("workflow_type").value = tpl.workflow_type;

  // Try to select version from dropdown, fall back gracefully
  const verSel = document.getElementById("workflow_type_version");
  verSel.value = tpl.workflow_type_version;
  // If value didn't match any option, add it temporarily
  if (verSel.value !== tpl.workflow_type_version) {
    const opt = document.createElement("option");
    opt.value = tpl.workflow_type_version;
    opt.textContent = tpl.workflow_type_version;
    verSel.appendChild(opt);
    verSel.value = tpl.workflow_type_version;
  }

  if (tpl.workflow_params && Object.keys(tpl.workflow_params).length > 0) {
    setEditorHtml("wp", tokenize(JSON.stringify(tpl.workflow_params, null, 2)));
  }
  if (tpl.tags && Object.keys(tpl.tags).length > 0) {
    setEditorHtml("tags", tokenize(JSON.stringify(tpl.tags, null, 2)));
  }
}

// ── JSON editor helpers ────────────────────────────────────────────────────

function escHtml(s) {
  return String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
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
    showEditorError(id);
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

function showEditorError(id) {
  const wrap = document.getElementById(`${id}-wrap`);
  // Re-trigger animation by removing and re-adding the class
  wrap.classList.remove("is-invalid");
  void wrap.offsetWidth; // force reflow
  wrap.classList.add("is-invalid");
}

function clearEditorError(id) {
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

// ── Form field validation ──────────────────────────────────────────────────

function getField(id) {
  return document.getElementById(id).value.trim();
}

function setInvalid(id) {
  document.getElementById(id).classList.add("is-invalid");
}

function clearInvalid(id) {
  document.getElementById(id).classList.remove("is-invalid");
  const err = document.getElementById(`${id}_err`);
  if (err) err.textContent = "";
}

// ── Form actions ───────────────────────────────────────────────────────────

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
  if (loading) {
    spinner.classList.add("is-spinning");
    label.textContent = "Submitting…";
  } else {
    spinner.classList.remove("is-spinning");
    label.textContent = "Submit Run";
  }
}

async function _submitRun(e) {
  e.preventDefault();
  clearFormError();

  const required = ["workflow_url", "workflow_type", "workflow_type_version"];
  let valid = true;

  required.forEach((id) => {
    clearInvalid(id);
    if (!getField(id)) {
      setInvalid(id);
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
      showEditorError(id);
      valid = false;
    }
  });

  if (!valid) {
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
  if (registeredEngines.length === 1) {
    document.getElementById("workflow_engine_select").value = registeredEngines[0].name;
    onEngineChange();
  } else {
    document.getElementById("workflow_engine").value = "";
    document.getElementById("workflow_engine_version").value = "";
    populateWorkflowTypes(null);
    populateWorkflowTypeVersions(null);
  }
  ["workflow_url", "workflow_type", "workflow_type_version"].forEach(clearInvalid);
  ["wp", "tags", "ep"].forEach((id) => {
    clearEditor(id);
    clearEditorError(id);
  });
  clearFormError();
  document.getElementById("success-banner-wrap").innerHTML = "";
}

// ── Event wiring ───────────────────────────────────────────────────────────

document.getElementById("workflow_engine_select").addEventListener("change", onEngineChange);

// ── Public aliases for HTML onclick handlers ───────────────────────────────

window.submitRun = (e) => _submitRun(e);
window.resetForm = () => _resetForm();
window.formatEditor = (id) => _formatEditor(id);

// ── Init ───────────────────────────────────────────────────────────────────

loadServiceInfo();
