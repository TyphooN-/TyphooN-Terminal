/**
 * TyphooN-Terminal Floating Window Manager
 *
 * Draggable, resizable, tileable floating panels — inspired by Godel Terminal.
 * Each window can display: articles, fundamentals, SEC filings, earnings, screener, etc.
 */

let windowZIndex = 1000;
let activeWindows = {};
let windowIdCounter = 0;

/**
 * Create a floating window.
 * @param {Object} opts
 * @param {string} opts.title - Window title
 * @param {string} opts.type - Window type (article, fundamentals, filing, earnings, screener, custom)
 * @param {number} [opts.width=500] - Initial width
 * @param {number} [opts.height=400] - Initial height
 * @param {number} [opts.x] - Initial X position (default: centered)
 * @param {number} [opts.y] - Initial Y position (default: centered with offset)
 * @param {string} [opts.content] - HTML content (text only, sanitized)
 * @param {Function} [opts.onClose] - Callback when closed
 * @returns {Object} Window handle { id, element, setContent, setTitle, close }
 */
export function createWindow(opts) {
  const id = `tw-${++windowIdCounter}`;
  const w = opts.width || 500;
  const h = opts.height || 400;

  // Cascade position for multiple windows
  const cascade = Object.keys(activeWindows).length * 30;
  const x = opts.x ?? Math.max(50, (window.innerWidth - w) / 2 + cascade);
  const y = opts.y ?? Math.max(50, 100 + cascade);

  // Create window DOM
  const win = document.createElement("div");
  win.id = id;
  win.className = "floating-window";
  win.style.width = w + "px";
  win.style.height = h + "px";
  win.style.left = x + "px";
  win.style.top = y + "px";
  win.style.zIndex = ++windowZIndex;

  // Title bar
  const titleBar = document.createElement("div");
  titleBar.className = "fw-titlebar";

  const titleText = document.createElement("span");
  titleText.className = "fw-title";
  titleText.textContent = opts.title || "Window";

  const controls = document.createElement("div");
  controls.className = "fw-controls";

  const btnMin = document.createElement("button");
  btnMin.className = "fw-btn fw-btn-min";
  btnMin.textContent = "−";
  btnMin.title = "Minimize";

  const btnMax = document.createElement("button");
  btnMax.className = "fw-btn fw-btn-max";
  btnMax.textContent = "□";
  btnMax.title = "Maximize";

  const btnClose = document.createElement("button");
  btnClose.className = "fw-btn fw-btn-close";
  btnClose.textContent = "×";
  btnClose.title = "Close";

  controls.appendChild(btnMin);
  controls.appendChild(btnMax);
  controls.appendChild(btnClose);
  titleBar.appendChild(titleText);
  titleBar.appendChild(controls);

  // Content area
  const content = document.createElement("div");
  content.className = "fw-content";
  if (opts.content) content.textContent = opts.content;

  // Resize handle
  const resizeHandle = document.createElement("div");
  resizeHandle.className = "fw-resize";

  win.appendChild(titleBar);
  win.appendChild(content);
  win.appendChild(resizeHandle);

  document.getElementById("app").appendChild(win);

  // ── Dragging ──────────────────────────────────────────────
  let isDragging = false, dragOffsetX = 0, dragOffsetY = 0;

  titleBar.addEventListener("mousedown", (e) => {
    if (e.target.classList.contains("fw-btn")) return;
    isDragging = true;
    dragOffsetX = e.clientX - win.offsetLeft;
    dragOffsetY = e.clientY - win.offsetTop;
    win.style.zIndex = ++windowZIndex;
    win.classList.add("dragging");
    e.preventDefault();
  });

  const onDragMove = (e) => {
    if (!isDragging) return;
    let newX = e.clientX - dragOffsetX;
    let newY = e.clientY - dragOffsetY;
    newX = Math.max(0, Math.min(newX, window.innerWidth - 100));
    newY = Math.max(0, Math.min(newY, window.innerHeight - 50));
    win.style.left = newX + "px";
    win.style.top = newY + "px";

    win.classList.remove("snap-left", "snap-right", "snap-top");
    if (e.clientX < 20) win.classList.add("snap-left");
    else if (e.clientX > window.innerWidth - 20) win.classList.add("snap-right");
    else if (e.clientY < 20) win.classList.add("snap-top");
  };

  const onDragUp = (e) => {
    if (!isDragging) return;
    isDragging = false;
    win.classList.remove("dragging");

    if (win.classList.contains("snap-left")) {
      win.style.left = "0"; win.style.top = "0";
      win.style.width = "50vw"; win.style.height = "100vh";
      win.classList.remove("snap-left");
    } else if (win.classList.contains("snap-right")) {
      win.style.left = "50vw"; win.style.top = "0";
      win.style.width = "50vw"; win.style.height = "100vh";
      win.classList.remove("snap-right");
    } else if (win.classList.contains("snap-top")) {
      win.style.left = "0"; win.style.top = "0";
      win.style.width = "100vw"; win.style.height = "100vh";
      win.classList.remove("snap-top");
    }
  };

  document.addEventListener("mousemove", onDragMove);
  document.addEventListener("mouseup", onDragUp);

  // ── Resizing ──────────────────────────────────────────────
  let isResizing = false, resizeStartX = 0, resizeStartY = 0, startW = 0, startH = 0;

  resizeHandle.addEventListener("mousedown", (e) => {
    isResizing = true;
    resizeStartX = e.clientX;
    resizeStartY = e.clientY;
    startW = win.offsetWidth;
    startH = win.offsetHeight;
    win.style.zIndex = ++windowZIndex;
    e.preventDefault();
    e.stopPropagation();
  });

  const onResizeMove = (e) => {
    if (!isResizing) return;
    const newW = Math.max(250, startW + (e.clientX - resizeStartX));
    const newH = Math.max(150, startH + (e.clientY - resizeStartY));
    win.style.width = newW + "px";
    win.style.height = newH + "px";
  };

  const onResizeUp = () => {
    isResizing = false;
  };

  document.addEventListener("mousemove", onResizeMove);
  document.addEventListener("mouseup", onResizeUp);

  // ── Controls ──────────────────────────────────────────────
  let isMinimized = false, isMaximized = false;
  let preMaxState = {};

  btnMin.addEventListener("click", () => {
    isMinimized = !isMinimized;
    content.style.display = isMinimized ? "none" : "";
    resizeHandle.style.display = isMinimized ? "none" : "";
    if (isMinimized) {
      win.style.height = "auto";
    } else {
      win.style.height = h + "px";
    }
  });

  btnMax.addEventListener("click", () => {
    if (!isMaximized) {
      preMaxState = { left: win.style.left, top: win.style.top, width: win.style.width, height: win.style.height };
      win.style.left = "0";
      win.style.top = "0";
      win.style.width = "100vw";
      win.style.height = "100vh";
      isMaximized = true;
      btnMax.textContent = "❐";
    } else {
      Object.assign(win.style, preMaxState);
      isMaximized = false;
      btnMax.textContent = "□";
    }
  });

  btnClose.addEventListener("click", () => {
    // Clean up document-level listeners to prevent leaks
    document.removeEventListener("mousemove", onDragMove);
    document.removeEventListener("mouseup", onDragUp);
    document.removeEventListener("mousemove", onResizeMove);
    document.removeEventListener("mouseup", onResizeUp);
    win.remove();
    delete activeWindows[id];
    if (opts.onClose) opts.onClose();
  });

  // Click to bring to front
  win.addEventListener("mousedown", () => {
    win.style.zIndex = ++windowZIndex;
  });

  // ── Handle ────────────────────────────────────────────────
  const handle = {
    id,
    element: win,
    contentElement: content,
    setContent(text) {
      content.textContent = "";
      if (typeof text === "string") {
        content.textContent = text;
      }
    },
    appendElement(el) {
      content.appendChild(el);
    },
    setTitle(t) {
      titleText.textContent = t;
    },
    close() {
      btnClose.click();
    },
  };

  activeWindows[id] = handle;
  return handle;
}

/**
 * Create a window displaying article content.
 */
export function openArticleWindow(title, paragraphs) {
  const win = createWindow({
    title: title.substring(0, 60),
    type: "article",
    width: 550,
    height: 500,
  });

  for (const text of paragraphs) {
    const p = document.createElement("p");
    p.textContent = text;
    p.style.margin = "8px 0";
    p.style.lineHeight = "1.6";
    win.appendElement(p);
  }

  return win;
}

/**
 * Create a window displaying fundamentals data.
 */
export function openFundamentalsWindow(symbol, data) {
  const win = createWindow({
    title: `${symbol} — Fundamentals`,
    type: "fundamentals",
    width: 400,
    height: 350,
  });

  const fmtNum = (v) => {
    if (!v || !v.value) return "N/A";
    const n = Number(v.value);
    if (Math.abs(n) >= 1e12) return `$${(n / 1e12).toFixed(2)}T`;
    if (Math.abs(n) >= 1e9) return `$${(n / 1e9).toFixed(2)}B`;
    if (Math.abs(n) >= 1e6) return `$${(n / 1e6).toFixed(2)}M`;
    return `$${n.toLocaleString()}`;
  };

  const rows = [
    ["Entity", data.entity || "—"],
    ["Revenue", fmtNum(data.revenue)],
    ["Net Income", fmtNum(data.net_income)],
    ["Total Assets", fmtNum(data.total_assets)],
    ["Total Liabilities", fmtNum(data.total_liabilities)],
    ["Stockholders' Equity", fmtNum(data.stockholders_equity)],
    ["Shares Outstanding", data.shares_outstanding?.value ? Number(data.shares_outstanding.value).toLocaleString() : "N/A"],
    ["EPS", data.eps?.value ? `$${Number(data.eps.value).toFixed(2)}` : "N/A"],
  ];

  const table = document.createElement("table");
  table.className = "fw-table";
  for (const [label, value] of rows) {
    const tr = document.createElement("tr");
    const td1 = document.createElement("td");
    td1.textContent = label;
    td1.className = "fw-label";
    const td2 = document.createElement("td");
    td2.textContent = value;
    td2.className = "fw-value";
    tr.appendChild(td1);
    tr.appendChild(td2);
    table.appendChild(tr);
  }

  win.appendElement(table);
  return win;
}

/**
 * Create a window displaying SEC filings.
 */
export function openFilingsWindow(symbol, filings) {
  const win = createWindow({
    title: `${symbol} — SEC Filings`,
    type: "filings",
    width: 500,
    height: 400,
  });

  if (!filings || filings.length === 0) {
    win.setContent("No filings found.");
    return win;
  }

  for (const filing of filings) {
    const item = document.createElement("div");
    item.className = "fw-filing-item";

    const type = document.createElement("span");
    type.className = "fw-filing-type";
    type.textContent = filing.form || filing.type || "Filing";

    const date = document.createElement("span");
    date.className = "fw-filing-date";
    date.textContent = filing.filed || filing.date || "";

    const desc = document.createElement("span");
    desc.className = "fw-filing-desc";
    desc.textContent = filing.description || filing.title || "";

    item.appendChild(type);
    item.appendChild(date);
    item.appendChild(desc);
    win.appendElement(item);
  }

  return win;
}

/**
 * Tile all open windows in a grid.
 */
export function tileWindows() {
  const ids = Object.keys(activeWindows);
  if (ids.length === 0) return;

  const cols = Math.ceil(Math.sqrt(ids.length));
  const rows = Math.ceil(ids.length / cols);
  const w = window.innerWidth / cols;
  const h = window.innerHeight / rows;

  ids.forEach((id, i) => {
    const win = activeWindows[id].element;
    const col = i % cols;
    const row = Math.floor(i / cols);
    win.style.left = col * w + "px";
    win.style.top = row * h + "px";
    win.style.width = w + "px";
    win.style.height = h + "px";
  });
}

/**
 * Close all floating windows.
 */
export function closeAllWindows() {
  for (const id of Object.keys(activeWindows)) {
    activeWindows[id].close();
  }
}

/**
 * Get count of open windows.
 */
export function getWindowCount() {
  return Object.keys(activeWindows).length;
}
