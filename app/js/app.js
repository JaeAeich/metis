const API_BASE = "/api";
let currentRunId = null;
let logsEventSource = null;
let logsPaused = false;
let _nextPageToken = null;

const CANCELABLE_STATES = new Set(["QUEUED", "INITIALIZING", "RUNNING", "PAUSED"]);

// ── Helpers ───────────────────────────────────────────────────────────────

function formatTimestamp(ts) {
  if (!ts) return "-";
  return new Date(ts).toLocaleString();
}

function formatRelativeTime(ts) {
  if (!ts) return "-";
  const diff = Date.now() - new Date(ts).getTime();
  const s = Math.floor(diff / 1000);
  const m = Math.floor(s / 60);
  const h = Math.floor(m / 60);
  if (s < 60) return `${s}s ago`;
  if (m < 60) return `${m}m ago`;
  if (h < 24) return `${h}h ago`;
  return formatTimestamp(ts);
}

function getStatusClass(state) {
  return state ? `status-${state}` : "";
}

function escapeHtml(str) {
  if (!str) return "";
  const d = document.createElement("div");
  d.textContent = str;
  return d.innerHTML;
}

function escapeAttr(str) {
  if (!str) return "";
  return String(str)
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function truncate(str, maxLen = 40) {
  if (!str || str.length <= maxLen) return escapeHtml(str);
  return `<span title="${escapeAttr(str)}" style="cursor:help;display:inline-block;max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;vertical-align:bottom;">${escapeHtml(str.substring(0, maxLen))}…</span>`;
}

// ── Runs list ─────────────────────────────────────────────────────────────

function filterRuns() {
  const state = document.getElementById("state-filter").value;
  document.querySelectorAll("#runs-body tr").forEach((row) => {
    row.style.display = !state || row.dataset.state === state ? "" : "none";
  });
}

function renderRunsList(data) {
  const tbody = document.getElementById("runs-body");
  if (!data.runs || data.runs.length === 0) {
    tbody.innerHTML = '<tr><td colspan="5" class="empty-state">No runs found</td></tr>';
    return;
  }

  _nextPageToken = data.next_page_token;

  tbody.innerHTML = data.runs
    .map((run) => {
      const state = run.state || "UNKNOWN";
      const canCancel = CANCELABLE_STATES.has(state);
      const safeId = escapeAttr(run.run_id);
      const cancelBtn = canCancel
        ? `<button
            class="btn btn-danger btn-danger-sm"
            onclick="event.stopPropagation(); cancelRunById('${safeId}', this)"
            title="Cancel run">Cancel</button>`
        : `<button class="btn btn-danger btn-danger-sm" disabled title="Cannot cancel">Cancel</button>`;

      return `
        <tr data-state="${state}" onclick="window.location.href='/run.html?run_id=${encodeURIComponent(run.run_id)}'">
          <td><span class="run-id">${escapeHtml(run.run_id)}</span></td>
          <td><span class="status-badge ${getStatusClass(state)}">${state}</span></td>
          <td>${formatRelativeTime(run.start_time)}</td>
          <td>${run.end_time ? formatRelativeTime(run.end_time) : "-"}</td>
          <td class="col-action">${cancelBtn}</td>
        </tr>`;
    })
    .join("");

  filterRuns();
}

async function _cancelRunById(runId, btn) {
  if (!confirm(`Cancel run ${runId}?\nThis cannot be undone.`)) return;
  btn.disabled = true;
  btn.textContent = "…";
  try {
    const res = await fetch(`${API_BASE}/runs/${encodeURIComponent(runId)}/cancel`, {
      method: "POST",
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      alert(`Cancel failed: ${err.msg || res.status}`);
      btn.disabled = false;
      btn.textContent = "Cancel";
      return;
    }
    btn.textContent = "Canceled";
    setTimeout(() => htmx.trigger("#runs-body", "load"), 800);
  } catch (err) {
    alert(`Cancel failed: ${err.message}`);
    btn.disabled = false;
    btn.textContent = "Cancel";
  }
}

// ── Run detail ────────────────────────────────────────────────────────────

async function _loadRun(runId) {
  currentRunId = runId;
  document.getElementById("run-id-display").textContent = runId;

  try {
    const [runRes, tasksRes] = await Promise.all([
      fetch(`${API_BASE}/runs/${encodeURIComponent(runId)}`),
      fetch(`${API_BASE}/runs/${encodeURIComponent(runId)}/tasks`),
    ]);
    if (!runRes.ok) throw new Error("Run not found");
    const run = await runRes.json();
    const tasks = await tasksRes.json();
    renderRunHeader(run);
    renderOverview(run);
    renderTasks(tasks);
  } catch (err) {
    document.getElementById("run-header").innerHTML =
      `<div class="empty-state">Error: ${escapeHtml(err.message)}</div>`;
  }
}

function renderRunHeader(run) {
  const state = run.state || "UNKNOWN";
  const canCancel = CANCELABLE_STATES.has(state);
  document.getElementById("run-header").innerHTML = `
    <div class="run-header-top">
      <span class="status-badge ${getStatusClass(state)}">${state}</span>
      <button type="button" class="btn btn-danger" id="cancel-btn"
        onclick="cancelRun()" ${canCancel ? "" : "disabled"}>
        Cancel Run
      </button>
    </div>
    <div class="run-info">
      <div class="run-info-item">
        <label>Started</label>
        <span>${formatTimestamp(run.request?.start_time)}</span>
      </div>
      <div class="run-info-item">
        <label>Workflow URL</label>
        <span>${truncate(run.request?.workflow_url, 50)}</span>
      </div>
      <div class="run-info-item">
        <label>Workflow Type</label>
        <span>${escapeHtml(run.request?.workflow_type || "-")} ${escapeHtml(run.request?.workflow_type_version || "")}</span>
      </div>
    </div>`;
}

async function _cancelRun() {
  const btn = document.getElementById("cancel-btn");
  if (!btn || !currentRunId) return;
  if (!confirm("Cancel this run? This cannot be undone.")) return;
  btn.disabled = true;
  btn.textContent = "Canceling…";
  try {
    const res = await fetch(`${API_BASE}/runs/${encodeURIComponent(currentRunId)}/cancel`, {
      method: "POST",
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      alert(`Cancel failed: ${err.msg || res.status}`);
      btn.disabled = false;
      btn.textContent = "Cancel Run";
      return;
    }
    setTimeout(() => _loadRun(currentRunId), 1000);
  } catch (err) {
    alert(`Cancel failed: ${err.message}`);
    btn.disabled = false;
    btn.textContent = "Cancel Run";
  }
}

function renderOverview(run) {
  const tags = run.request?.tags || {};
  const tagsHtml =
    Object.keys(tags).length > 0
      ? Object.entries(tags)
          .map(
            ([k, v]) =>
              `<div style="margin-bottom:0.25rem;"><strong>${truncate(k, 30)}:</strong> ${truncate(String(v), 50)}</div>`
          )
          .join("")
      : '<div class="empty-state">No tags</div>';

  document.getElementById("overview-content").innerHTML = `
    <div class="run-info">
      <div class="run-info-item">
        <label>Run ID</label>
        <span class="run-id">${escapeHtml(currentRunId)}</span>
      </div>
      <div class="run-info-item">
        <label>Workflow Engine</label>
        <span>${escapeHtml(run.request?.workflow_engine || "-")}</span>
      </div>
      <div class="run-info-item">
        <label>Engine Version</label>
        <span>${escapeHtml(run.request?.workflow_engine_version || "-")}</span>
      </div>
    </div>
    <h3 style="margin-top:1.5rem;margin-bottom:0.5rem;">Tags</h3>
    <div>${tagsHtml}</div>`;
}

function renderTasks(data) {
  const tbody = document.getElementById("tasks-body");
  if (!data.task_logs || data.task_logs.length === 0) {
    tbody.innerHTML = '<tr><td colspan="5" class="empty-state">No tasks found</td></tr>';
    return;
  }

  tbody.innerHTML = data.task_logs
    .map((task) => {
      const exitCode = task.exit_code;
      let status = "RUNNING";
      if (exitCode !== null && exitCode !== undefined) {
        status = exitCode === 0 ? "COMPLETE" : "EXECUTOR_ERROR";
      }
      return `
        <tr>
          <td><span class="run-id">${escapeHtml(task.id)}</span></td>
          <td>${escapeHtml(task.name || "-")}</td>
          <td><span class="status-badge ${getStatusClass(status)}">${status}</span></td>
          <td>${formatRelativeTime(task.start_time)}</td>
          <td>${task.end_time ? formatRelativeTime(task.end_time) : "-"}</td>
        </tr>`;
    })
    .join("");
}

// ── Tabs ──────────────────────────────────────────────────────────────────

function _showTab(tabName, btn) {
  document.querySelectorAll(".tab-content").forEach((el) => {
    el.classList.remove("active");
  });
  document.querySelectorAll(".tab-btn").forEach((el) => {
    el.classList.remove("active");
  });
  document.getElementById(`tab-${tabName}`).classList.add("active");
  btn.classList.add("active");
  if (tabName === "logs" && !logsEventSource) connectLogs();
}

// ── Logs ──────────────────────────────────────────────────────────────────

function connectLogs() {
  if (logsEventSource) logsEventSource.close();
  const streamFilter = document.getElementById("log-stream-filter").value;
  let url = `${API_BASE}/runs/${encodeURIComponent(currentRunId)}/logs/stream`;
  if (streamFilter) url += `?stream=${encodeURIComponent(streamFilter)}`;
  logsEventSource = new EventSource(url);

  logsEventSource.onmessage = (event) => {
    if (logsPaused) return;
    try {
      const log = JSON.parse(event.data);
      const container = document.getElementById("logs-container");
      const line = document.createElement("div");
      line.className = `log-line ${log.stream}`;
      line.innerHTML = `
        <span class="log-timestamp">${formatTimestamp(log.written_at)}</span>
        <span class="log-stream">[${log.stream}]</span>
        <span class="log-content">${escapeHtml(log.line)}</span>`;
      container.appendChild(line);
      if (container.scrollHeight - container.scrollTop <= container.clientHeight + 100) {
        container.scrollTop = container.scrollHeight;
      }
    } catch (e) {
      console.error("Failed to parse log line:", e);
    }
  };

  logsEventSource.onerror = () => {
    console.log("SSE connection lost, reconnecting...");
    setTimeout(connectLogs, 3000);
  };
}

function _toggleLogs() {
  logsPaused = !logsPaused;
  document.getElementById("logs-pause-btn").textContent = logsPaused ? "Resume" : "Pause";
}

function clearLogs() {
  document.getElementById("logs-container").innerHTML = "";
}

document.getElementById("log-stream-filter")?.addEventListener("change", () => {
  clearLogs();
  connectLogs();
});

// ── htmx hook ─────────────────────────────────────────────────────────────

document.body.addEventListener("htmx:beforeSwap", (evt) => {
  if (evt.detail.target.id === "runs-body") {
    try {
      const data = JSON.parse(evt.detail.xhr.responseText);
      evt.detail.shouldSwap = false;
      renderRunsList(data);
    } catch (e) {
      console.error("Failed to parse runs response:", e);
    }
  }
});

// ── Exports ───────────────────────────────────────────────────────────────

window.loadRun = _loadRun;
window.cancelRun = _cancelRun;
window.showTab = _showTab;
window.toggleLogs = _toggleLogs;
