const RANGES = ["60", "900", "3600", "21600", "86400", "604800"];
const savedRange = localStorage.getItem("monitor.range");
let range = RANGES.includes(savedRange) ? savedRange : "60";
let series = { cpu: [], gpu: [], power: [] };
let historyAt = 0;
const STRINGS = window.STRINGS;

const savedLang = localStorage.getItem("monitor.lang");
let lang = savedLang === "en" || savedLang === "zh"
  ? savedLang
  : (navigator.language || "").toLowerCase().startsWith("en") ? "en" : "zh";
let lastData = null;
let lastOk = null;
let lastError = "";

function t(key, vars = {}) {
  const template = (STRINGS[lang] && STRINGS[lang][key]) || STRINGS.zh[key] || key;
  return template.replace(/\{(\w+)\}/g, (_, name) => (vars[name] == null ? "" : vars[name]));
}

function applyStatic() {
  document.documentElement.lang = lang === "en" ? "en" : "zh-CN";
  document.querySelectorAll("[data-i18n]").forEach((node) => {
    node.textContent = t(node.dataset.i18n);
  });
  document.querySelectorAll("[data-i18n-aria]").forEach((node) => {
    node.setAttribute("aria-label", t(node.dataset.i18nAria));
  });
  document.querySelectorAll(".lang button").forEach((button) => {
    const selected = button.dataset.lang === lang;
    button.classList.toggle("active", selected);
    button.setAttribute("aria-pressed", selected ? "true" : "false");
  });
  document.querySelectorAll("#ranges button").forEach((button) => {
    const selected = button.dataset.range === range;
    button.classList.toggle("active", selected);
    button.setAttribute("aria-pressed", selected ? "true" : "false");
  });
  if (lastOk == null) {
    document.querySelector("#live-text").textContent = t("connecting");
  } else {
    setStatus(lastOk, lastError);
  }
}

function setLang(next) {
  if (!STRINGS[next] || next === lang) return;
  lang = next;
  localStorage.setItem("monitor.lang", lang);
  applyStatic();
  if (lastData) render(lastData);
  if (lastOk != null) setStatus(lastOk, lastError);
  if (zoomDialog.open) renderZoom();
}

function setStatus(ok, message) {
  lastOk = ok;
  lastError = message || "";
  live.classList.toggle("bad", !ok);
  document.querySelector("#live-text").textContent = t(ok ? "live" : "offline");
  errorBox.hidden = ok;
  if (!ok) errorBox.textContent = displayError(lastError);
}

function displayError(message) {
  if (message === "Sample failed") return t("sampleFailed");
  if (message === "Sampling") return t("warming");
  return message || t("sampleFailed");
}

const metrics = document.querySelector("#metrics");
const threads = document.querySelector("#threads");
const gpuDetail = document.querySelector("#gpu-detail");
const errorBox = document.querySelector("#error");
const live = document.querySelector("#live");
const cpuCanvas = document.querySelector("#cpu-chart");
const gpuCanvas = document.querySelector("#gpu-chart");

function fmtBytes(bytes) {
  if (bytes == null) return "—";
  const gib = bytes / 1024 ** 3;
  return `${gib.toFixed(gib >= 10 ? 1 : 2)} GB`;
}

function fmtPct(value) {
  return value == null ? "—" : value.toFixed(1);
}

function fmtUptime(seconds) {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  if (days > 0) return t("uptimeDays", { days, hours });
  if (hours > 0) return t("uptimeHours", { hours, mins });
  return t("uptimeMins", { mins });
}

function metricCard(kind, label, value, unit, detail, percent) {
  const width = percent == null ? 0 : Math.max(0, Math.min(100, percent));
  return `
    <article class="metric">
      <div class="label"><span>${label}</span></div>
      <strong class="${kind}">${value}<em>${unit}</em></strong>
      <small>${detail}</small>
      <div class="bar"><i class="${kind}" style="width:${width}%;background:currentColor"></i></div>
    </article>`;
}

function render(data) {
  const cpu = data.cpu;
  const mem = data.memory;
  const gpu = data.gpus[0];
  const temp = cpu.temp && cpu.temp.primary;
  document.querySelector("#host").textContent = data.host;
  document.querySelector("#subtitle").textContent = t("subtitle", {
    model: cpu.model,
    physical: cpu.physical || "?",
    logical: cpu.logical,
    uptime: fmtUptime(data.uptime_s),
  });

  const memDetail = t("memDetail", { used: fmtBytes(mem.used), total: fmtBytes(mem.total) })
    + (mem.swap_total ? ` · ${t("swap", { percent: fmtPct(mem.swap_percent) })}` : "");
  const cards = [
    metricCard(
      "cpu",
      "CPU",
      fmtPct(cpu.percent),
      "%",
      `${cpu.freq_mhz ? cpu.freq_mhz + " MHz" : t("freqUnknown")} · ${temp ? temp.label + " " + temp.celsius + "°C" : t("tempUnavailable")}`,
      cpu.percent
    ),
    metricCard("mem", t("memory"), fmtPct(mem.percent), "%", memDetail, mem.percent),
  ];

  if (gpu) {
    const memPct = gpu.mem_total_mib ? (gpu.mem_used_mib / gpu.mem_total_mib) * 100 : null;
    cards.push(
      metricCard(
        "gpu",
        gpu.name.replace("NVIDIA ", ""),
        fmtPct(gpu.util),
        "%",
        t("vramDetail", {
          temp: gpu.temp_c ?? "—",
          used: gpu.mem_used_mib,
          total: gpu.mem_total_mib,
        }),
        gpu.util
      )
    );
    if (gpu.power_w != null) {
      const powerPct = gpu.power_limit_w ? (gpu.power_w / gpu.power_limit_w) * 100 : null;
      cards.push(
        metricCard(
          "power",
          t("gpuPower"),
          Number(gpu.power_w).toFixed(1),
          "W",
          gpu.power_limit_w
            ? t("powerLimit", { limit: Number(gpu.power_limit_w).toFixed(0) })
            : t("noPowerLimit"),
          powerPct
        )
      );
    } else {
      cards.push(metricCard("power", t("gpuPower"), "—", "", t("noPower"), null));
    }
  } else {
    cards.push(metricCard("gpu", "GPU", "—", "", data.gpu_error || t("noNvidia"), null));
    cards.push(metricCard("power", t("gpuPower"), "—", "", t("noGpuData"), null));
  }
  metrics.innerHTML = cards.join("");

  document.querySelector("#thread-meta").textContent = t("threadMeta", {
    load: cpu.load.join(" / "),
    freq: cpu.freq_mhz || "—",
  });
  threads.innerHTML = cpu.threads.map((thread) => {
    const pct = thread.percent == null ? 0 : thread.percent;
    return `
      <div class="thread">
        <b>CPU ${thread.id}</b>
        <div class="bar"><i class="cpu" style="width:${pct}%;background:currentColor"></i></div>
        <span>${thread.percent == null ? "—" : thread.percent.toFixed(0) + "%"} · ${thread.mhz || "—"}</span>
      </div>`;
  }).join("");

  if (!data.gpus.length) {
    gpuDetail.innerHTML = `<p class="note">${data.gpu_error || t("noNvidiaSentence")}</p>`;
  } else {
    gpuDetail.innerHTML = data.gpus.map((item) => {
      const powerPct = item.power_w != null && item.power_limit_w
        ? Math.max(0, Math.min(100, item.power_w / item.power_limit_w * 100))
        : 0;
      const powerText = item.power_w == null
        ? t("noDriverPower")
        : `${Number(item.power_w).toFixed(1)} W / ${Number(item.power_limit_w).toFixed(0)} W`;
      const fan = item.fan_pct == null ? "—" : item.fan_pct === 0 ? t("stopped") : `${item.fan_pct}%`;
      return `
        <div class="gpu-card">
          <div class="gpu-name">${item.name}</div>
          <dl class="stats">
            <div class="stat"><dt>${t("usage")}</dt><dd class="gpu">${fmtPct(item.util)}%</dd></div>
            <div class="stat"><dt>${t("vramUtil")}</dt><dd>${item.mem_util ?? "—"}%</dd></div>
            <div class="stat"><dt>${t("temperature")}</dt><dd>${item.temp_c ?? "—"}°C</dd></div>
            <div class="stat"><dt>${t("fan")}</dt><dd>${fan}</dd></div>
            <div class="stat"><dt>${t("coreClock")}</dt><dd>${item.clock_mhz ?? "—"} MHz</dd></div>
            <div class="stat"><dt>${t("memClock")}</dt><dd>${item.mem_clock_mhz ?? "—"} MHz</dd></div>
          </dl>
          <div class="power-track">
            <div class="label"><span>${t("power")}</span><span>${powerText}</span></div>
            <div class="bar"><i class="power" style="width:${powerPct}%;background:currentColor"></i></div>
          </div>
        </div>`;
    }).join("");
  }

  const powerLabel = gpu && gpu.power_w != null
    ? `${Number(gpu.power_w).toFixed(1)} W`
    : t("powerUnavailable");
  document.querySelector("#cpu-chart-label").textContent = t("cpuChart", {
    percent: fmtPct(cpu.percent),
    range: t("range" + range),
  });
  document.querySelector("#gpu-chart-label").textContent = gpu
    ? t("gpuChart", { percent: fmtPct(gpu.util), power: powerLabel, range: t("range" + range) })
    : t("noGpuChart");
}

function drawChart(canvas, series) {
  const dpr = window.devicePixelRatio || 1;
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  const padLeft = 28;
  const padBottom = 4;
  canvas.width = Math.max(1, Math.floor(width * dpr));
  canvas.height = Math.max(1, Math.floor(height * dpr));
  const ctx = canvas.getContext("2d");
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, width, height);

  ctx.font = "11px Noto Sans CJK SC, sans-serif";
  ctx.fillStyle = "rgba(163,155,136,0.9)";
  ctx.textBaseline = "middle";
  [100, 50, 0].forEach((tick) => {
    const y = ((100 - tick) / 100) * (height - padBottom - 8) + 4;
    ctx.fillText(String(tick), 0, y);
    ctx.strokeStyle = "rgba(244,236,214,0.08)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(padLeft, Math.round(y) + 0.5);
    ctx.lineTo(width, Math.round(y) + 0.5);
    ctx.stroke();
  });

  const plotWidth = width - padLeft;
  const end = Date.now();
  const start = end - Number(range) * 1000;
  const span = Math.max(1, end - start);
  series.forEach((item) => {
    const points = item.values
      .filter((point) => point.value != null && point.ts >= start && point.ts <= end)
      .map((point) => [
        padLeft + ((point.ts - start) / span) * plotWidth,
        ((100 - point.value) / 100) * (height - padBottom - 8) + 4,
      ]);
    if (!points.length) return;
    ctx.beginPath();
    points.forEach(([x, y], index) => {
      if (index === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.strokeStyle = item.color;
    ctx.lineWidth = 2;
    ctx.stroke();
    const last = points[points.length - 1];
    ctx.fillStyle = item.color;
    ctx.beginPath();
    ctx.arc(last[0], last[1], 3.2, 0, Math.PI * 2);
    ctx.fill();
  });
}

function paintCharts() {
  drawChart(cpuCanvas, [{ color: "#d6f27a", values: series.cpu }]);
  drawChart(gpuCanvas, [
    { color: "#8fd0c6", values: series.gpu },
    { color: "#f0997b", values: series.power },
  ]);
}

async function loadHistory(force) {
  const wait = Number(range) <= 900 ? 1000 : 5000;
  if (!force && Date.now() - historyAt < wait) return;
  historyAt = Date.now();
  const response = await fetch(`/api/history?seconds=${range}`, { cache: "no-store" });
  const data = await response.json();
  if (!response.ok || !data.ok) throw new Error(data.error || "history");
  series = {
    cpu: data.points.map((point) => ({ ts: point.ts, value: point.cpu })),
    gpu: data.points.map((point) => ({ ts: point.ts, value: point.gpu })),
    power: data.points.map((point) => ({ ts: point.ts, value: point.power })),
  };
  paintCharts();
}

async function tick() {
  try {
    const response = await fetch("/api/stats", { cache: "no-store" });
    const data = await response.json();
    if (!response.ok || !data.ok) throw new Error(data.error || "Sample failed");
    lastData = data;
    render(data);
    setStatus(true);
  } catch (error) {
    setStatus(false, error.message);
  }
  try {
    await loadHistory(false);
  } catch (error) {
    if (!lastOk) setStatus(false, error.message);
  }
}

applyStatic();
document.querySelector(".lang").addEventListener("click", (event) => {
  const button = event.target.closest("button[data-lang]");
  if (button) setLang(button.dataset.lang);
});
document.querySelector("#ranges").addEventListener("click", (event) => {
  const button = event.target.closest("button[data-range]");
  if (!button || button.dataset.range === range) return;
  range = button.dataset.range;
  localStorage.setItem("monitor.range", range);
  historyAt = 0;
  applyStatic();
  if (lastData) render(lastData);
  loadHistory(true);
});
const zoomDialog = document.querySelector("#zoom");
const zoomCanvas = document.querySelector("#zoom-chart");
const zoomTip = document.querySelector("#zoom-tip");
const zoomState = { metric: "cpu", start: 0, end: 0, points: [], step: 1, drag: null };

function pad(value) {
  return String(value).padStart(2, "0");
}

function toLocalInput(ms) {
  const date = new Date(ms);
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

function formatTime(ms, span) {
  const date = new Date(ms);
  const clock = `${pad(date.getHours())}:${pad(date.getMinutes())}`;
  const full = `${clock}:${pad(date.getSeconds())}`;
  const day = `${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
  if (span < 3 * 60 * 1000) return full;
  if (span < 24 * 3600 * 1000) return clock;
  return `${day} ${clock}`;
}

function formatStamp(ms) {
  const date = new Date(ms);
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

function summarize(values) {
  const nums = values.filter((value) => value != null && !Number.isNaN(value));
  if (!nums.length) return null;
  const total = nums.reduce((sum, value) => sum + value, 0);
  return {
    avg: total / nums.length,
    min: Math.min(...nums),
    max: Math.max(...nums),
  };
}

function summaryCard(label, value) {
  return `<div><dl><dt>${label}</dt><dd>${value}</dd></dl></div>`;
}

function num(value, digits, suffix) {
  if (value == null || Number.isNaN(value)) return "—";
  return `${Number(value).toFixed(digits)}${suffix || ""}`;
}

function renderZoom() {
  const metric = zoomState.metric;
  document.querySelector("#zoom-title").textContent = metric === "cpu" ? "CPU" : "GPU";
  document.querySelector("#zoom-start").value = toLocalInput(zoomState.start);
  document.querySelector("#zoom-end").value = toLocalInput(zoomState.end);
  const note = zoomState.points.length
    ? t("stepNote", { step: zoomState.step })
    : t("noSamples");
  document.querySelector("#zoom-note").textContent = note;

  const usage = summarize(zoomState.points.map((point) => point[metric]));
  const cards = [
    summaryCard(t("avgUsage"), num(usage && usage.avg, 1, "%")),
    summaryCard(t("maxUsage"), num(usage && usage.max, 1, "%")),
    summaryCard(t("minUsage"), num(usage && usage.min, 1, "%")),
  ];
  if (metric === "cpu") {
    const freq = summarize(zoomState.points.map((point) => point.cpu_mhz));
    const temp = summarize(zoomState.points.map((point) => point.cpu_temp));
    cards.push(summaryCard(t("avgFreq"), num(freq && freq.avg, 0, " MHz")));
    cards.push(summaryCard(t("avgTemp"), num(temp && temp.avg, 1, "°C")));
  } else {
    const power = summarize(zoomState.points.map((point) => point.power_w));
    const temp = summarize(zoomState.points.map((point) => point.gpu_temp));
    cards.push(summaryCard(t("avgPower"), num(power && power.avg, 1, " W")));
    cards.push(summaryCard(t("maxPower"), num(power && power.max, 1, " W")));
    cards.push(summaryCard(t("avgTemp"), num(temp && temp.avg, 1, "°C")));
  }
  document.querySelector("#zoom-summary").innerHTML = cards.join("");
  document.querySelector("#zoom-legend").innerHTML = metric === "cpu"
    ? `<span class="cpu"><i></i>${t("usage")}</span>`
    : `<span class="gpu"><i></i>${t("usage")}</span><span class="power"><i></i>${t("power")}</span>`;
  drawZoom();
}

function zoomBox() {
  return {
    width: zoomCanvas.clientWidth,
    height: zoomCanvas.clientHeight,
    left: 44,
    right: zoomState.metric === "gpu" ? 48 : 16,
    top: 16,
    bottom: 36,
  };
}

function zoomX(ts, box) {
  const span = Math.max(1, zoomState.end - zoomState.start);
  const plot = box.width - box.left - box.right;
  return box.left + ((ts - zoomState.start) / span) * plot;
}

function zoomTime(x, box) {
  const plot = Math.max(1, box.width - box.left - box.right);
  const ratio = Math.min(1, Math.max(0, (x - box.left) / plot));
  return zoomState.start + ratio * (zoomState.end - zoomState.start);
}

function drawZoom() {
  const box = zoomBox();
  const dpr = window.devicePixelRatio || 1;
  zoomCanvas.width = Math.max(1, Math.floor(box.width * dpr));
  zoomCanvas.height = Math.max(1, Math.floor(box.height * dpr));
  const ctx = zoomCanvas.getContext("2d");
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, box.width, box.height);
  ctx.font = "11px Noto Sans CJK SC, sans-serif";
  ctx.textBaseline = "middle";
  ctx.fillStyle = "rgba(163,155,136,0.95)";
  const plotBottom = box.height - box.bottom;
  const plotHeight = plotBottom - box.top;
  [100, 50, 0].forEach((tick) => {
    const y = box.top + ((100 - tick) / 100) * plotHeight;
    ctx.fillText(String(tick), 0, y);
    ctx.strokeStyle = "rgba(244,236,214,0.08)";
    ctx.beginPath();
    ctx.moveTo(box.left, Math.round(y) + 0.5);
    ctx.lineTo(box.width - box.right, Math.round(y) + 0.5);
    ctx.stroke();
  });

  const powerMax = Math.max(
    1,
    ...zoomState.points.map((point) => point.power_w || 0)
  );
  if (zoomState.metric === "gpu") {
    ctx.textAlign = "right";
    [powerMax, powerMax / 2, 0].forEach((tick, index) => {
      const y = box.top + (index / 2) * plotHeight;
      ctx.fillText(`${Math.round(tick)}W`, box.width - 2, y);
    });
    ctx.textAlign = "left";
  }

  const span = zoomState.end - zoomState.start;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  for (let index = 0; index <= 4; index += 1) {
    const ts = zoomState.start + (span * index) / 4;
    const x = zoomX(ts, box);
    ctx.fillText(formatTime(ts, span), x, plotBottom + 8);
  }
  ctx.textAlign = "left";
  ctx.textBaseline = "middle";

  const line = (key, color, scale) => {
    const points = zoomState.points
      .filter((point) => point[key] != null)
      .map((point) => [zoomX(point.ts, box), box.top + (1 - scale(point[key])) * plotHeight]);
    if (!points.length) return;
    ctx.beginPath();
    points.forEach(([x, y], index) => (index ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
    ctx.strokeStyle = color;
    ctx.lineWidth = 2;
    ctx.stroke();
  };
  if (zoomState.metric === "cpu") {
    line("cpu", "#d6f27a", (value) => value / 100);
  } else {
    line("gpu", "#8fd0c6", (value) => value / 100);
    line("power_w", "#f0997b", (value) => value / powerMax);
  }

  if (zoomState.drag) {
    const left = Math.min(zoomState.drag.x0, zoomState.drag.x1);
    const right = Math.max(zoomState.drag.x0, zoomState.drag.x1);
    ctx.fillStyle = "rgba(214,242,122,0.16)";
    ctx.fillRect(left, box.top, Math.max(1, right - left), plotHeight);
  }
}

function tipText(point) {
  const rows = [formatStamp(point.ts)];
  if (zoomState.metric === "cpu") {
    rows.push(`${t("usage")} ${num(point.cpu, 1, "%")}`);
    rows.push(`${t("frequency")} ${num(point.cpu_mhz, 0, " MHz")}`);
    rows.push(`${t("temperature")} ${num(point.cpu_temp, 1, "°C")}`);
    rows.push(`${t("memory")} ${num(point.mem, 1, "%")}`);
  } else {
    rows.push(`${t("usage")} ${num(point.gpu, 1, "%")}`);
    rows.push(`${t("power")} ${num(point.power_w, 1, " W")}`);
    rows.push(`${t("temperature")} ${num(point.gpu_temp, 1, "°C")}`);
  }
  return rows.join("<br>");
}

function nearestPoint(ms) {
  let best = null;
  let distance = Infinity;
  zoomState.points.forEach((point) => {
    const gap = Math.abs(point.ts - ms);
    if (gap < distance) {
      distance = gap;
      best = point;
    }
  });
  return best;
}

async function loadZoom() {
  const response = await fetch(
    `/api/history?start=${Math.round(zoomState.start)}&end=${Math.round(zoomState.end)}`,
    { cache: "no-store" }
  );
  const data = await response.json();
  if (!response.ok || !data.ok) throw new Error(data.error || "history");
  zoomState.start = data.start;
  zoomState.end = data.end;
  zoomState.step = data.step_s;
  zoomState.points = data.points;
  renderZoom();
}

async function openZoom(metric) {
  zoomState.metric = metric;
  zoomState.end = Date.now();
  zoomState.start = zoomState.end - Number(range) * 1000;
  zoomState.drag = null;
  zoomTip.hidden = true;
  if (!zoomDialog.open) zoomDialog.showModal();
  try {
    await loadZoom();
  } catch (error) {
    document.querySelector("#zoom-note").textContent = error.message === "Invalid time range"
      ? t("badRange")
      : error.message;
  }
}

document.querySelectorAll(".zoom-open[data-zoom]").forEach((button) => {
  button.addEventListener("click", () => openZoom(button.dataset.zoom));
});
document.querySelector("#zoom-close").addEventListener("click", () => zoomDialog.close());
zoomDialog.addEventListener("click", (event) => {
  if (event.target === zoomDialog) zoomDialog.close();
});
document.querySelector("#zoom-filter").addEventListener("submit", async (event) => {
  event.preventDefault();
  const start = new Date(document.querySelector("#zoom-start").value).getTime();
  const end = new Date(document.querySelector("#zoom-end").value).getTime();
  if (!start || !end || end - start < 5000) {
    document.querySelector("#zoom-note").textContent = t("badRange");
    return;
  }
  zoomState.start = start;
  zoomState.end = end;
  try {
    await loadZoom();
  } catch (error) {
    document.querySelector("#zoom-note").textContent = error.message === "Invalid time range"
      ? t("badRange")
      : error.message;
  }
});

zoomCanvas.addEventListener("mousedown", (event) => {
  const rect = zoomCanvas.getBoundingClientRect();
  const x = event.clientX - rect.left;
  const box = zoomBox();
  if (x < box.left || x > box.width - box.right) return;
  zoomState.drag = { x0: x, x1: x };
  zoomTip.hidden = true;
});
window.addEventListener("mousemove", (event) => {
  if (!zoomDialog.open) return;
  const rect = zoomCanvas.getBoundingClientRect();
  const x = event.clientX - rect.left;
  const y = event.clientY - rect.top;
  const box = zoomBox();
  if (zoomState.drag) {
    zoomState.drag.x1 = Math.min(box.width - box.right, Math.max(box.left, x));
    drawZoom();
    return;
  }
  if (x < box.left || x > box.width - box.right || y < 0 || y > rect.height) {
    zoomTip.hidden = true;
    return;
  }
  const point = nearestPoint(zoomTime(x, box));
  if (!point) {
    zoomTip.hidden = true;
    return;
  }
  zoomTip.hidden = false;
  zoomTip.innerHTML = tipText(point);
  const left = Math.min(x + 14, rect.width - zoomTip.offsetWidth - 8);
  zoomTip.style.left = `${Math.max(8, left)}px`;
  zoomTip.style.top = `${Math.max(8, y - 28)}px`;
});
window.addEventListener("mouseup", async () => {
  if (!zoomState.drag) return;
  const drag = zoomState.drag;
  zoomState.drag = null;
  const box = zoomBox();
  const start = zoomTime(Math.min(drag.x0, drag.x1), box);
  const end = zoomTime(Math.max(drag.x0, drag.x1), box);
  if (end - start < 5000) {
    drawZoom();
    return;
  }
  zoomState.start = start;
  zoomState.end = end;
  try {
    await loadZoom();
  } catch (error) {
    document.querySelector("#zoom-note").textContent = error.message === "Invalid time range"
      ? t("badRange")
      : error.message;
  }
});

tick();
setInterval(tick, 1000);
window.addEventListener("resize", () => {
  paintCharts();
  if (zoomDialog.open) drawZoom();
});
