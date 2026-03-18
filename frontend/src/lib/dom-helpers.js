// dom-helpers.js — Safe DOM construction (innerHTML elimination layer)
// All user-facing DOM construction MUST use these instead of innerHTML.

export function el(tag, style, text) {
  const e = document.createElement(tag);
  if (style) e.style.cssText = style;
  if (text !== undefined && text !== null) e.textContent = String(text);
  return e;
}

export function span(text, style) { return el("span", style, text); }
export function div(text, style) { return el("div", style, text); }

export function td(text, style) { return el("td", style, text); }

export function theadRow(headers, style) {
  const tr = document.createElement("tr");
  const s = style || "padding:4px;color:#888;font-weight:bold;border-bottom:1px solid #333;";
  for (const h of headers) tr.appendChild(el("td", s, h));
  return tr;
}

export function styledRow(values) {
  const tr = document.createElement("tr");
  for (const v of values) tr.appendChild(td(v.text || v.t || "", v.style || v.s || "padding:4px;"));
  return tr;
}

export function colorSpan(text, color) { return span(text, `color:${color};`); }

export function labelValue(label, value, valueColor) {
  const container = document.createElement("span");
  container.appendChild(span(label, "color:#888;"));
  container.appendChild(span(String(value), `color:${valueColor || "#ccc"};font-weight:bold;`));
  return container;
}

export function setText(id, text) {
  const el = document.getElementById(id);
  if (el && el.textContent !== text) el.textContent = text;
}

export function setTextClass(id, text, cls) {
  const el = document.getElementById(id);
  if (!el) return;
  if (el.textContent !== text) el.textContent = text;
  const full = "dash-row " + cls;
  if (el.className !== full) el.className = full;
}
