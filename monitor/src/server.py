#!/usr/bin/env python3
"""Local CPU, memory, and GPU monitor page.

Usage:
  python3 src/server.py
  python3 src/server.py 8780
  docker compose up --build

Open http://127.0.0.1:8780 in a browser.
A direct run listens on localhost only. Docker mounts the host /proc, /sys,
and nvidia-smi into the container, so the page shows the host. When power
cannot be read, the page skips it.
"""

from __future__ import annotations

import csv
import json
import os
import socket
import sqlite3
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "src"
STATIC_DIRS = {
    "src": SRC,
    "assets": ROOT / "assets",
    "i18n": ROOT / "i18n",
}
PROC = os.environ.get("HOST_PROC", "/proc").rstrip("/") or "/proc"
SYS = os.environ.get("HOST_SYS", "/sys").rstrip("/") or "/sys"
BIND = os.environ.get("BIND", "127.0.0.1")
DATA_DIR = Path(os.environ.get("DATA_DIR", ROOT / "data"))
DB_PATH = DATA_DIR / "monitor.sqlite"
SAMPLE_INTERVAL = 1.0
RETENTION_S = 7 * 24 * 3600
# Aggregation step in seconds for each query window. Short windows keep one sample per second; longer windows use averages.
RANGES = {
    60: 1,
    900: 1,
    3600: 5,
    21600: 30,
    86400: 120,
    604800: 900,
}
STATIC_TYPES = {
    ".html": "text/html; charset=utf-8",
    ".css": "text/css; charset=utf-8",
    ".js": "text/javascript; charset=utf-8",
    ".svg": "image/svg+xml",
    ".ico": "image/x-icon",
}


def read_text(path: str) -> str:
    with open(path, encoding="utf-8") as handle:
        return handle.read()


def cpu_identity() -> tuple[str, int]:
    model = "CPU"
    cores: set[tuple[str, str]] = set()
    physical = core = None
    for line in read_text(f"{PROC}/cpuinfo").splitlines():
        if line.startswith("model name") and model == "CPU":
            model = line.split(":", 1)[1].strip()
        elif line.startswith("physical id"):
            physical = line.split(":", 1)[1].strip()
        elif line.startswith("core id"):
            core = line.split(":", 1)[1].strip()
        elif not line.strip() and physical is not None and core is not None:
            cores.add((physical, core))
            physical = core = None
    if physical is not None and core is not None:
        cores.add((physical, core))
    return model, len(cores)


def logical_layout() -> list[dict]:
    threads: list[dict] = []
    current: dict = {}
    for line in read_text(f"{PROC}/cpuinfo").splitlines():
        if line.startswith("processor"):
            if current:
                threads.append(current)
            current = {"id": int(line.split(":", 1)[1])}
        elif line.startswith("core id") and current:
            current["core"] = int(line.split(":", 1)[1])
    if current:
        threads.append(current)
    return threads


def read_cpu_times() -> list[tuple[str, int, int]]:
    samples = []
    for line in read_text(f"{PROC}/stat").splitlines():
        if not line.startswith("cpu"):
            break
        parts = line.split()
        values = [int(item) for item in parts[1:]]
        idle = values[3] + (values[4] if len(values) > 4 else 0)
        samples.append((parts[0], sum(values), idle))
    return samples


def cpu_percents(previous, current) -> list[float] | None:
    if not previous or len(previous) != len(current):
        return None
    percents = []
    for (_, total0, idle0), (_, total1, idle1) in zip(previous, current):
        total = total1 - total0
        idle = idle1 - idle0
        if total <= 0:
            percents.append(0.0)
        else:
            percents.append(round(max(0.0, min(100.0, (1 - idle / total) * 100)), 1))
    return percents


def read_frequencies(count: int) -> list[float | None]:
    freqs = []
    for index in range(count):
        path = f"{SYS}/devices/system/cpu/cpu{index}/cpufreq/scaling_cur_freq"
        try:
            freqs.append(round(int(read_text(path).strip()) / 1000, 0))
        except OSError:
            freqs.append(None)
    return freqs


def read_meminfo() -> dict[str, int]:
    info = {}
    for line in read_text(f"{PROC}/meminfo").splitlines():
        key, _, rest = line.partition(":")
        info[key] = int(rest.strip().split()[0]) * 1024
    return info


def read_cpu_temp() -> dict | None:
    hwmon = Path(SYS) / "class" / "hwmon"
    if not hwmon.is_dir():
        return None
    for chip in hwmon.iterdir():
        try:
            name = (chip / "name").read_text(encoding="utf-8").strip()
        except OSError:
            continue
        if name not in {"k10temp", "coretemp"}:
            continue
        readings = []
        for label_path in sorted(chip.glob("temp*_label")):
            input_path = chip / label_path.name.replace("_label", "_input")
            try:
                label = label_path.read_text(encoding="utf-8").strip()
                celsius = int(input_path.read_text(encoding="utf-8").strip()) / 1000
            except (OSError, ValueError):
                continue
            readings.append({"label": label, "celsius": round(celsius, 1)})
        if not readings:
            continue
        primary = next(
            (item for item in readings if item["label"] in {"Tctl", "Package id 0"}),
            readings[0],
        )
        return {"primary": primary, "readings": readings}
    return None


def read_gpus() -> tuple[list[dict], str | None]:
    query = [
        "index",
        "name",
        "utilization.gpu",
        "utilization.memory",
        "memory.used",
        "memory.total",
        "temperature.gpu",
        "power.draw",
        "power.limit",
        "clocks.gr",
        "clocks.mem",
        "fan.speed",
    ]
    try:
        completed = subprocess.run(
            [
                "nvidia-smi",
                f"--query-gpu={','.join(query)}",
                "--format=csv,noheader,nounits",
            ],
            capture_output=True,
            text=True,
            timeout=2,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return [], str(exc)
    if completed.returncode != 0:
        message = (completed.stderr or completed.stdout or "nvidia-smi 不可用").strip()
        return [], message.splitlines()[0]

    gpus = []
    for row in csv.reader(completed.stdout.splitlines()):
        cells = [cell.strip() for cell in row]
        if len(cells) < len(query):
            continue
        record = dict(zip(query, cells))
        gpus.append(
            {
                "index": int(record["index"]),
                "name": record["name"],
                "util": number(record["utilization.gpu"]),
                "mem_util": number(record["utilization.memory"]),
                "mem_used_mib": number(record["memory.used"]),
                "mem_total_mib": number(record["memory.total"]),
                "temp_c": number(record["temperature.gpu"]),
                "power_w": number(record["power.draw"]),
                "power_limit_w": number(record["power.limit"]),
                "clock_mhz": number(record["clocks.gr"]),
                "mem_clock_mhz": number(record["clocks.mem"]),
                "fan_pct": number(record["fan.speed"]),
            }
        )
    return gpus, None


def number(raw: str):
    try:
        value = float(raw)
    except (TypeError, ValueError):
        return None
    if value.is_integer():
        return int(value)
    return round(value, 2)


def read_load() -> list[float]:
    parts = read_text(f"{PROC}/loadavg").split()
    return [round(float(part), 2) for part in parts[:3]]


def read_uptime() -> float:
    return float(read_text(f"{PROC}/uptime").split()[0])


def machine_name() -> str:
    explicit = os.environ.get("HOST_NAME", "").strip()
    if explicit:
        return explicit
    path = os.environ.get("HOST_HOSTNAME_FILE", "").strip()
    if path:
        try:
            name = Path(path).read_text(encoding="utf-8").strip()
        except OSError:
            name = ""
        if name:
            return name
    return socket.gethostname()


class History:
    def __init__(self) -> None:
        DATA_DIR.mkdir(parents=True, exist_ok=True)
        self.lock = threading.Lock()
        self.conn = sqlite3.connect(DB_PATH, check_same_thread=False)
        self.conn.execute("PRAGMA journal_mode=WAL")
        self.conn.execute(
            """
            CREATE TABLE IF NOT EXISTS samples (
                ts INTEGER PRIMARY KEY,
                cpu_pct REAL,
                cpu_mhz REAL,
                cpu_temp REAL,
                mem_pct REAL,
                gpu_pct REAL,
                gpu_power_w REAL,
                gpu_power_limit_w REAL,
                gpu_temp REAL
            )
            """
        )
        self.conn.commit()
        self.writes = 0

    def record(self, snapshot: dict) -> None:
        cpu = snapshot.get("cpu") or {}
        if cpu.get("percent") is None:
            return
        memory = snapshot.get("memory") or {}
        gpu = (snapshot.get("gpus") or [{}])[0]
        temp = ((cpu.get("temp") or {}).get("primary") or {}).get("celsius")
        row = (
            int(snapshot["ts"] * 1000),
            cpu.get("percent"),
            cpu.get("freq_mhz"),
            temp,
            memory.get("percent"),
            gpu.get("util"),
            gpu.get("power_w"),
            gpu.get("power_limit_w"),
            gpu.get("temp_c"),
        )
        with self.lock:
            self.conn.execute(
                """
                INSERT OR REPLACE INTO samples (
                    ts, cpu_pct, cpu_mhz, cpu_temp, mem_pct,
                    gpu_pct, gpu_power_w, gpu_power_limit_w, gpu_temp
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                row,
            )
            self.writes += 1
            if self.writes % 60 == 0:
                cutoff = int((time.time() - RETENTION_S) * 1000)
                self.conn.execute("DELETE FROM samples WHERE ts < ?", (cutoff,))
            self.conn.commit()

    def query(self, seconds: int) -> dict:
        end = int(time.time() * 1000)
        payload = self.query_between(end - seconds * 1000, end, RANGES[seconds])
        payload["seconds"] = seconds
        return payload

    def query_between(self, start: int, end: int, step: int | None = None) -> dict:
        span_s = max(1, int((end - start) / 1000))
        step = step or choose_step(span_s)
        bucket = step * 1000
        with self.lock:
            rows = self.conn.execute(
                """
                SELECT (ts / ?) * ?,
                       AVG(cpu_pct), AVG(cpu_mhz), AVG(cpu_temp), AVG(mem_pct),
                       AVG(gpu_pct), AVG(gpu_power_w), AVG(gpu_power_limit_w), AVG(gpu_temp)
                FROM samples
                WHERE ts >= ? AND ts <= ?
                GROUP BY 1
                ORDER BY 1
                """,
                (bucket, bucket, start, end),
            ).fetchall()
        points = []
        for ts, cpu, mhz, cpu_temp, mem, gpu, power, limit_w, gpu_temp in rows:
            power_pct = None
            if power is not None and limit_w:
                power_pct = round(power / limit_w * 100, 1)
            points.append(
                {
                    "ts": ts,
                    "cpu": round1(cpu),
                    "cpu_mhz": round1(mhz),
                    "cpu_temp": round1(cpu_temp),
                    "mem": round1(mem),
                    "gpu": round1(gpu),
                    "power_w": round1(power),
                    "power": power_pct,
                    "gpu_temp": round1(gpu_temp),
                }
            )
        return {
            "ok": True,
            "start": start,
            "end": end,
            "step_s": step,
            "points": points,
        }


def choose_step(span_s: int) -> int:
    if span_s <= 15 * 60:
        step = 1
    elif span_s <= 60 * 60:
        step = 5
    elif span_s <= 6 * 3600:
        step = 30
    elif span_s <= 24 * 3600:
        step = 120
    else:
        step = 900
    while span_s / step > 800:
        step *= 2
    return step


def round1(value):
    if value is None:
        return None
    return round(float(value), 1)


HISTORY = History()


class Sampler:
    def __init__(self) -> None:
        self.lock = threading.Lock()
        self.snapshot: dict | None = None
        self.previous_cpu = None
        self.model, self.physical_cores = cpu_identity()
        self.layout = logical_layout()

    def loop(self) -> None:
        while True:
            started = time.monotonic()
            try:
                snapshot = self.collect()
                HISTORY.record(snapshot)
                with self.lock:
                    self.snapshot = snapshot
            except Exception as exc:
                with self.lock:
                    self.snapshot = {
                        "ok": False,
                        "error": str(exc),
                        "ts": time.time(),
                    }
            elapsed = time.monotonic() - started
            time.sleep(max(0.05, SAMPLE_INTERVAL - elapsed))

    def collect(self) -> dict:
        cpu_times = read_cpu_times()
        percents = cpu_percents(self.previous_cpu, cpu_times)
        self.previous_cpu = cpu_times
        logical = max(0, len(cpu_times) - 1)
        per_core = percents[1:] if percents else [None] * logical
        freqs = read_frequencies(logical)
        known_freqs = [value for value in freqs if value is not None]
        threads = []
        for index, item in enumerate(self.layout[:logical]):
            threads.append(
                {
                    "id": item.get("id", index),
                    "core": item.get("core"),
                    "percent": per_core[index] if index < len(per_core) else None,
                    "mhz": freqs[index] if index < len(freqs) else None,
                }
            )
        mem = read_meminfo()
        total = mem.get("MemTotal", 0)
        available = mem.get("MemAvailable", 0)
        used = max(0, total - available)
        swap_total = mem.get("SwapTotal", 0)
        swap_free = mem.get("SwapFree", 0)
        gpus, gpu_error = read_gpus()
        temp = read_cpu_temp()
        return {
            "ok": True,
            "ts": time.time(),
            "host": machine_name(),
            "uptime_s": round(read_uptime()),
            "cpu": {
                "model": self.model,
                "logical": logical,
                "physical": self.physical_cores,
                "percent": percents[0] if percents else None,
                "freq_mhz": round(sum(known_freqs) / len(known_freqs)) if known_freqs else None,
                "temp": temp,
                "load": read_load(),
                "threads": threads,
            },
            "memory": {
                "total": total,
                "used": used,
                "available": available,
                "percent": round(used / total * 100, 1) if total else None,
                "swap_total": swap_total,
                "swap_used": max(0, swap_total - swap_free),
                "swap_percent": round((swap_total - swap_free) / swap_total * 100, 1)
                if swap_total
                else None,
            },
            "gpus": gpus,
            "gpu_error": gpu_error,
        }


SAMPLER = Sampler()


class Handler(BaseHTTPRequestHandler):
    def do_GET(self) -> None:
        parsed = urlparse(self.path)
        try:
            if parsed.path == "/api/stats":
                self.send_stats()
                return
            if parsed.path == "/api/history":
                self.send_history(parsed)
                return
            self.serve_static(parsed.path)
        except (BrokenPipeError, ConnectionResetError):
            return

    def send_stats(self) -> None:
        with SAMPLER.lock:
            payload = SAMPLER.snapshot
        if payload is None:
            self.send_json(503, {"ok": False, "error": "Sampling"})
            return
        self.send_json(200 if payload.get("ok", True) else 500, payload)

    def send_history(self, parsed) -> None:
        query = parse_qs(parsed.query)
        if "start" in query or "end" in query:
            try:
                start = int(query.get("start", ["0"])[0])
                end = int(query.get("end", ["0"])[0])
            except ValueError:
                self.send_json(400, {"ok": False, "error": "Invalid time range"})
                return
            now = int(time.time() * 1000)
            if end > now:
                end = now
            earliest = now - RETENTION_S * 1000
            if start < earliest:
                start = earliest
            if end - start < 5000 or end - start > RETENTION_S * 1000:
                self.send_json(400, {"ok": False, "error": "Invalid time range"})
                return
            self.send_json(200, HISTORY.query_between(start, end))
            return
        raw = query.get("seconds", ["60"])[0]
        try:
            seconds = int(raw)
        except ValueError:
            seconds = 0
        if seconds not in RANGES:
            self.send_json(400, {"ok": False, "error": "Unknown time range"})
            return
        self.send_json(200, HISTORY.query(seconds))

    def send_json(self, status: int, payload: dict) -> None:
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def serve_static(self, url_path: str) -> None:
        if url_path in {"", "/"}:
            path = SRC / "index.html"
            base = SRC
        else:
            parts = [part for part in url_path.split("/") if part not in {"", "."}]
            if not parts or parts[0] not in STATIC_DIRS:
                self.send_error(404)
                return
            base = STATIC_DIRS[parts[0]]
            path = base.joinpath(*parts[1:]).resolve()
        if not path.is_file() or path != base and base not in path.parents:
            self.send_error(404)
            return
        body = path.read_bytes()
        self.send_response(200)
        self.send_header("Content-Type", STATIC_TYPES.get(path.suffix, "application/octet-stream"))
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt: str, *args) -> None:
        if args and (
            str(args[0]).startswith("GET /api/stats") or str(args[0]).startswith("GET /api/history")
        ):
            return
        super().log_message(fmt, *args)


def main() -> None:
    import sys

    port = int(os.environ.get("PORT") or (sys.argv[1] if len(sys.argv) > 1 else 8780))
    threading.Thread(target=SAMPLER.loop, name="sampler", daemon=True).start()
    server = ThreadingHTTPServer((BIND, port), Handler)
    shown = "127.0.0.1" if BIND in {"0.0.0.0", "::"} else BIND
    print(f"Resource monitor http://{shown}:{port}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nStopped", flush=True)
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
