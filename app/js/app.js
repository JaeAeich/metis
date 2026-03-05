const API_BASE = "/api";
let currentRunId = null;
let logsEventSource = null;
let statusEventSource = null;
let logsPaused = false;
let _nextPageToken = null;

const paginationState = {
  currentPage: 1,
  totalPages: 1,
  pageSize: 20,
  pageTokens: { 1: null },
  isLoading: false,
};

const CANCELABLE_STATES = new Set(["QUEUED", "INITIALIZING", "RUNNING", "PAUSED"]);
const DELETABLE_STATES = new Set([
  "COMPLETE",
  "EXECUTOR_ERROR",
  "SYSTEM_ERROR",
  "CANCELED",
  "PREEMPTED",
]);

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

function _truncate(str, maxLen = 40) {
  if (!str || str.length <= maxLen) return escapeHtml(str);
  return `<span title="${escapeAttr(str)}" style="cursor:help;display:inline-block;max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;vertical-align:bottom;">${escapeHtml(str.substring(0, maxLen))}…</span>`;
}

function calculatePageSize() {
  const vh = window.innerHeight;
  const headerHeight = 180;
  const paginationHeight = 80;
  const tableHeaderHeight = 50;
  const rowHeight = 52;
  const buffer = 20;
  const available = vh - headerHeight - paginationHeight - tableHeaderHeight - buffer;
  const rows = Math.floor(available / rowHeight);
  return Math.max(5, Math.min(30, rows));
}

function calculateDuration(start, end) {
  if (!start) return "-";
  const startTime = new Date(start);
  const endTime = end ? new Date(end) : new Date();
  const diffMs = endTime - startTime;
  const diffMins = Math.floor(diffMs / 60000);
  const diffSecs = Math.floor((diffMs % 60000) / 1000);
  if (diffMins === 0) return `${diffSecs}s`;
  if (diffSecs < 60) return `${diffMins}m ${diffSecs}s`;
}

function _formatJsonSection(data, title) {
  const jsonStr = typeof data === "string" ? data : JSON.stringify(data, null, 2);
  return `<details class="json-details">
    <summary>${title}</summary>
    <pre class="json-viewer"><code>${escapeHtml(jsonStr)}</code></pre>
  </details>`;
}

async function loadRunsPage(pageNum = 1) {
  if (paginationState.isLoading) return;

  paginationState.isLoading = true;
  const tbody = document.getElementById("runs-body");
  tbody.innerHTML = '<tr><td colspan="5" class="loading">Loading...</td></tr>';

  try {
    const pageSize = paginationState.pageSize;
    const pageToken = paginationState.pageTokens[pageNum] || null;

    let url = `${API_BASE}/runs?page_size=${pageSize}`;
    if (pageToken) {
      url += `&page_token=${encodeURIComponent(pageToken)}`;
    }

    const res = await fetch(url);
    if (!res.ok) throw new Error(`Failed to fetch runs: ${res.status}`);

    const data = await res.json();

    if (data.next_page_token) {
      paginationState.pageTokens[pageNum + 1] = data.next_page_token;
      paginationState.totalPages = Math.max(paginationState.totalPages, pageNum + 1);
    } else {
      paginationState.totalPages = pageNum;
    }

    paginationState.currentPage = pageNum;
    renderRunsList(data);
    renderPagination();
  } catch (err) {
    tbody.innerHTML = `<tr><td colspan="5" class="empty-state">Error: ${escapeHtml(err.message)}</td></tr>`;
  } finally {
    paginationState.isLoading = false;
  }
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
      const canDelete = DELETABLE_STATES.has(state);
      const safeId = escapeAttr(run.run_id);

      const cancelBtn = canCancel
        ? `<button
            class="btn btn-cancel btn-cancel-sm"
            onclick="event.stopPropagation(); cancelRunById('${safeId}', this)"
            title="Cancel the running workflow">Cancel</button>`
        : "";

      const deleteBtn = canDelete
        ? `<button
            class="btn btn-delete btn-delete-sm"
            onclick="event.stopPropagation(); deleteRunById('${safeId}', this)"
            title="Permanently delete this run from the database">Delete</button>`
        : "";

      return `
        <tr data-state="${state}" onclick="window.location.href='/run.html?run_id=${encodeURIComponent(run.run_id)}'">
          <td><span class="run-id">${escapeHtml(run.run_id)}</span></td>
          <td><span class="status-badge ${getStatusClass(state)}">${state}</span></td>
          <td>${formatRelativeTime(run.start_time)}</td>
          <td>${run.end_time ? formatRelativeTime(run.end_time) : "-"}</td>
          <td class="col-action">${cancelBtn}${deleteBtn}</td>
        </tr>`;
    })
    .join("");

  filterRuns();
}

function renderPagination() {
  const container = document.getElementById("pagination");
  const { currentPage, totalPages } = paginationState;

  if (totalPages <= 1) {
    container.innerHTML = '<span class="pagination-info">Page 1 of 1</span>';
    return;
  }

  const pages = generatePageNumbers(currentPage, totalPages);

  let html = `<button onclick="loadRunsPage(${currentPage - 1})" ${currentPage === 1 ? "disabled" : ""}>Previous</button>`;

  pages.forEach((page) => {
    if (page === "...") {
      html += '<span class="pagination-ellipsis">...</span>';
    } else {
      html += `<button class="${page === currentPage ? "active" : ""}" onclick="loadRunsPage(${page})">${page}</button>`;
    }
  });

  html += `<button onclick="loadRunsPage(${currentPage + 1})" ${currentPage === totalPages ? "disabled" : ""}>Next</button>`;

  html += `<span class="pagination-info">Page ${currentPage} of ${totalPages}</span>`;

  container.innerHTML = html;
}

function generatePageNumbers(current, total) {
  if (total <= 7) {
    return Array.from({ length: total }, (_, i) => i + 1);
  }

  if (current <= 3) {
    return [1, 2, 3, 4, "...", total];
  }

  if (current >= total - 2) {
    return [1, "...", total - 3, total - 2, total - 1, total];
  }

  return [1, "...", current - 1, current, current + 1, "...", total];
}

async function _cancelRunById(runId, btn) {
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
    setTimeout(() => loadRunsPage(paginationState.currentPage), 800);
  } catch (err) {
    alert(`Cancel failed: ${err.message}`);
    btn.disabled = false;
    btn.textContent = "Cancel";
  }
}

async function _deleteRunById(runId, btn) {
  btn.disabled = true;
  btn.textContent = "…";
  try {
    const res = await fetch(`${API_BASE}/runs/${encodeURIComponent(runId)}`, {
      method: "DELETE",
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      alert(`Delete failed: ${err.msg || res.status}`);
      btn.disabled = false;
      btn.textContent = "Delete";
      return;
    }
    setTimeout(() => htmx.trigger("#runs-body", "load"), 300);
  } catch (err) {
    alert(`Delete failed: ${err.message}`);
    btn.disabled = false;
    btn.textContent = "Delete";
  }
}

async function _deleteRun() {
  const btn = document.getElementById("delete-btn");
  if (!btn || !currentRunId) return;

  btn.disabled = true;
  btn.textContent = "Deleting…";

  try {
    const res = await fetch(`${API_BASE}/runs/${encodeURIComponent(currentRunId)}`, {
      method: "DELETE",
    });

    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      alert(`Delete failed: ${err.msg || res.status}`);
      btn.disabled = false;
      btn.textContent = "Delete Run";
      return;
    }

    window.location.href = "/";
  } catch (err) {
    alert(`Delete failed: ${err.message}`);
    btn.disabled = false;
    btn.textContent = "Delete Run";
  }
}
// ── Run detail ────────────────────────────────────────────────────────────

async function _loadRun(runId) {
  currentRunId = runId;
  const display = document.getElementById("run-id-display");
  if (display) {
    display.textContent = runId;
  }

  try {
    const runRes = await fetch(`${API_BASE}/runs/${encodeURIComponent(runId)}`);
    if (!runRes.ok) {
      const err = await runRes.json().catch(() => ({}));
      throw new Error(err.msg || `HTTP ${runRes.status}`);
    }
    const run = await runRes.json();
    renderRunHeader(run);
    renderOverview(run);
    renderTasks(tasks);
    connectStatusStream();
  } catch (err) {
    document.getElementById("overview-content").innerHTML =
      `<div class="empty-state">Error: ${escapeHtml(err.message)}</div>`;
    return;
  }

  try {
    const tasksRes = await fetch(`${API_BASE}/runs/${encodeURIComponent(runId)}/tasks`);
    if (tasksRes.ok) {
      const tasks = await tasksRes.json();
      renderTasks(tasks);
    }
  } catch (_) {
    // tasks are non-critical, silently ignore
  }
}

function renderRunHeader(run) {
  const state = run.state || "UNKNOWN";
  const canCancel = CANCELABLE_STATES.has(state);
  const canDelete = DELETABLE_STATES.has(state);

  const badge = document.getElementById("run-state-badge");
  if (badge) {
    badge.className = `status-badge ${getStatusClass(state)}`;
    badge.textContent = state;
  }

  const actions = document.getElementById("run-detail-actions");
  if (actions) {
    const cancelBtn = canCancel
      ? `<button type="button" class="btn btn-danger btn-sm" id="cancel-btn" onclick="cancelRun()">Cancel</button>`
      : "";
    const deleteBtn = canDelete
      ? `<button type="button" class="btn btn-delete btn-sm" id="delete-btn" onclick="deleteRun()">Delete</button>`
      : "";
    actions.innerHTML = cancelBtn + deleteBtn;
  }
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

function syntaxHighlightJson(obj) {
  const json = typeof obj === "string" ? obj : JSON.stringify(obj, null, 2);
  return escapeHtml(json).replace(
    /("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+-]?\d+)?)/g,
    (match) => {
      if (/^"/.test(match)) {
        if (/:$/.test(match)) return `<span class="json-key">${match}</span>`;
        return `<span class="json-str">${match}</span>`;
      }
      if (/true|false/.test(match)) return `<span class="json-bool">${match}</span>`;
      if (/null/.test(match)) return `<span class="json-null">${match}</span>`;
      return `<span class="json-num">${match}</span>`;
    }
  );
}

function syntaxHighlightCmd(cmd) {
  return escapeHtml(cmd).replace(
    /(\s)(--?[\w-][\w-]*)/g,
    (_, space, flag) => `${space}<span class="cmd-flag">${flag}</span>`
  );
}

function renderOverview(run) {
  const req = run.request || {};
  const tags = req.tags || {};
  const runLog = run.run_log || {};

  const tagsHtml =
    Object.keys(tags).length > 0
      ? Object.entries(tags)
          .map(
            ([k, v]) =>
              `<span class="tag-chip"><strong>${escapeHtml(k)}</strong>: ${escapeHtml(String(v))}</span>`
          )
          .join("")
      : '<span class="empty-state">No tags</span>';

  const cmdStr = runLog.cmd && runLog.cmd.length > 0 ? runLog.cmd.join(" ") : null;
  const cmdHtml = cmdStr
    ? `<div class="code-block-wrap">
        <button class="copy-btn" onclick="copyToClipboard(this, ${JSON.stringify(cmdStr)})">Copy</button>
        <pre class="cmd-viewer"><code>${syntaxHighlightCmd(cmdStr)}</code></pre>
      </div>`
    : '<span class="empty-state">Not available</span>';

  function jsonBlock(data) {
    const str = typeof data === "string" ? data : JSON.stringify(data, null, 2);
    return `<div class="code-block-wrap">
      <button class="copy-btn" onclick="copyToClipboard(this, ${JSON.stringify(str)})">Copy</button>
      <pre class="json-viewer"><code>${syntaxHighlightJson(data)}</code></pre>
    </div>`;
  }

  const workflowParamsHtml = req.workflow_params
    ? `<div class="overview-section"><p class="overview-section-label">Workflow Params</p>${jsonBlock(req.workflow_params)}</div>`
    : "";
  const engineParamsHtml = req.workflow_engine_parameters
    ? `<div class="overview-section"><p class="overview-section-label">Engine Parameters</p>${jsonBlock(req.workflow_engine_parameters)}</div>`
    : "";

  const urlHtml = req.workflow_url
    ? `<div class="run-url-row">
        <span class="overview-section-label">Workflow URL</span>
        <a class="run-url" href="${escapeAttr(req.workflow_url)}" target="_blank" rel="noopener">${escapeHtml(req.workflow_url)}</a>
      </div>`
    : "";

  document.getElementById("overview-content").innerHTML = `
    <div class="run-info">
      <div class="run-info-item">
        <label>Type</label>
        <span>${escapeHtml(req.workflow_type || "-")} <span class="meta-version">${escapeHtml(req.workflow_type_version || "")}</span></span>
      </div>
      <div class="run-info-item">
        <label>Engine</label>
        <span>${escapeHtml(req.workflow_engine || "-")} <span class="meta-version">${escapeHtml(req.workflow_engine_version || "")}</span></span>
      </div>
      <div class="run-info-item">
        <label>Started</label>
        <span>${formatTimestamp(runLog.start_time)}</span>
      </div>
      <div class="run-info-item">
        <label>Duration</label>
        <span>${calculateDuration(runLog.start_time, runLog.end_time)}</span>
      </div>
      <div class="run-info-item">
        <label>Exit Code</label>
        <span>${runLog.exit_code != null ? runLog.exit_code : "-"}</span>
      </div>
    </div>

    ${urlHtml}

    ${Object.keys(tags).length > 0 ? `<div class="overview-section"><p class="overview-section-label">Tags</p><div class="tag-chips">${tagsHtml}</div></div>` : ""}

    <div class="overview-section">
      <p class="overview-section-label">Command</p>
      ${cmdHtml}
    </div>

    ${workflowParamsHtml}
    ${engineParamsHtml}
  `;
}

function _copyToClipboard(btn, text) {
  navigator.clipboard.writeText(text).then(() => {
    btn.textContent = "Copied!";
    setTimeout(() => {
      btn.textContent = "Copy";
    }, 1500);
  });
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

function connectStatusStream() {
  if (statusEventSource) statusEventSource.close();
  const url = `${API_BASE}/runs/${encodeURIComponent(currentRunId)}/status/stream`;
  statusEventSource = new EventSource(url);

  statusEventSource.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      const badge = document.querySelector(".status-badge");
      if (badge) {
        badge.className = `status-badge ${getStatusClass(data.state)}`;
        badge.textContent = data.state;
      }

      if (DELETABLE_STATES.has(data.state)) {
        statusEventSource.close();
        _loadRun(currentRunId);
      }
    } catch (e) {
      console.error("Failed to parse status update:", e);
    }
  };

  statusEventSource.onerror = () => {
    console.log("Status SSE connection lost, reconnecting...");
    setTimeout(connectStatusStream, 3000);
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

document.addEventListener("DOMContentLoaded", () => {
  if (document.getElementById("runs-body")) {
    paginationState.pageSize = calculatePageSize();
    loadRunsPage(1);

    const handleResize = debounce(() => {
      const newSize = calculatePageSize();
      if (newSize !== paginationState.pageSize) {
        paginationState.pageSize = newSize;
        paginationState.pageTokens = { 1: null };
        loadRunsPage(1);
      }
    }, 250);

    window.addEventListener("resize", handleResize);

    setInterval(() => {
      if (!paginationState.isLoading) {
        loadRunsPage(paginationState.currentPage);
      }
    }, 10000);
  }
});

function debounce(fn, ms) {
  let timer;
  return (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  };
}

function filterRuns() {
  const state = document.getElementById("state-filter")?.value || "";
  document.querySelectorAll("#runs-body tr[data-state]").forEach((row) => {
    row.style.display = !state || row.dataset.state === state ? "" : "none";
  });
}

// ── Exports ───────────────────────────────────────────────────────────────

window.loadRun = _loadRun;
window.cancelRun = _cancelRun;
window.cancelRunById = _cancelRunById;
window.deleteRun = _deleteRun;
window.deleteRunById = _deleteRunById;
window.showTab = _showTab;
window.toggleLogs = _toggleLogs;
window.loadRunsPage = loadRunsPage;
