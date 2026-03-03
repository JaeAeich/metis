const API_BASE = "/api";
let currentRunId = null;
let logsEventSource = null;
let logsPaused = false;
let _nextPageToken = null;

function formatTimestamp(ts) {
  if (!ts) return "-";
  const date = new Date(ts);
  return date.toLocaleString();
}

function formatRelativeTime(ts) {
  if (!ts) return "-";
  const date = new Date(ts);
  const now = new Date();
  const diff = now - date;
  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (seconds < 60) return `${seconds}s ago`;
  if (minutes < 60) return `${minutes}m ago`;
  if (hours < 24) return `${hours}h ago`;
  return formatTimestamp(ts);
}

function getStatusClass(state) {
  if (!state) return "";
  return `status-${state}`;
}

function escapeHtml(str) {
  if (!str) return "";
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
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
  return `<span title="${escapeAttr(str)}" style="cursor: help; display: inline-block; max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; vertical-align: bottom;">${escapeHtml(str.substring(0, maxLen))}...</span>`;
}

function filterRuns() {
  const state = document.getElementById("state-filter").value;
  const rows = document.querySelectorAll("#runs-body tr");

  rows.forEach((row) => {
    if (!state || row.dataset.state === state) {
      row.style.display = "";
    } else {
      row.style.display = "none";
    }
  });
}

function renderRunsList(data) {
  const tbody = document.getElementById("runs-body");
  if (!data.runs || data.runs.length === 0) {
    tbody.innerHTML = '<tr><td colspan="4" class="empty-state">No runs found</td></tr>';
    return "";
  }

  _nextPageToken = data.next_page_token;

  const html = data.runs
    .map((run) => {
      const state = run.state || "UNKNOWN";
      return `
      <tr data-state="${state}" onclick="window.location.href='/run.html?run_id=${encodeURIComponent(run.run_id)}'">
        <td><span class="run-id">${escapeHtml(run.run_id)}</span></td>
        <td><span class="status-badge ${getStatusClass(state)}">${state}</span></td>
        <td>${formatRelativeTime(run.start_time)}</td>
        <td>${run.end_time ? formatRelativeTime(run.end_time) : "-"}</td>
      </tr>
    `;
    })
    .join("");

  tbody.innerHTML = html;
  filterRuns();
  return "";
}

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
      `<div class="empty-state">Error: ${err.message}</div>`;
  }
}

function renderRunHeader(run) {
  const state = run.state || "UNKNOWN";
  const html = `
    <div>
      <span class="status-badge ${getStatusClass(state)}">${state}</span>
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
    </div>
  `;
  document.getElementById("run-header").innerHTML = html;
}

function renderOverview(run) {
  const tags = run.request?.tags || {};
  const tagsHtml =
    Object.keys(tags).length > 0
      ? Object.entries(tags)
          .map(
            ([k, v]) =>
              `<div style="margin-bottom: 0.25rem;"><strong>${truncate(k, 30)}:</strong> ${truncate(String(v), 50)}</div>`
          )
          .join("")
      : '<div class="empty-state">No tags</div>';

  const html = `
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
    <h3 style="margin-top: 1.5rem; margin-bottom: 0.5rem;">Tags</h3>
    <div>${tagsHtml}</div>
  `;
  document.getElementById("overview-content").innerHTML = html;
}

function renderTasks(data) {
  const tbody = document.getElementById("tasks-body");
  if (!data.task_logs || data.task_logs.length === 0) {
    tbody.innerHTML = '<tr><td colspan="5" class="empty-state">No tasks found</td></tr>';
    return;
  }

  const html = data.task_logs
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
      </tr>
    `;
    })
    .join("");

  tbody.innerHTML = html;
}

function _showTab(tabName, btn) {
  document.querySelectorAll(".tab-content").forEach((el) => {
    el.classList.remove("active");
  });
  document.querySelectorAll(".tab-btn").forEach((el) => {
    el.classList.remove("active");
  });

  document.getElementById(`tab-${tabName}`).classList.add("active");
  btn.classList.add("active");

  if (tabName === "logs" && !logsEventSource) {
    connectLogs();
  }
}

function connectLogs() {
  if (logsEventSource) {
    logsEventSource.close();
  }

  const streamFilter = document.getElementById("log-stream-filter").value;
  let url = `${API_BASE}/runs/${encodeURIComponent(currentRunId)}/logs/stream`;
  if (streamFilter) {
    url += `?stream=${encodeURIComponent(streamFilter)}`;
  }

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
        <span class="log-content">${escapeHtml(log.line)}</span>
      `;

      container.appendChild(line);

      const shouldScroll =
        container.scrollHeight - container.scrollTop <= container.clientHeight + 100;
      if (shouldScroll) {
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

window.loadRun = _loadRun;
window.showTab = _showTab;
window.toggleLogs = _toggleLogs;
