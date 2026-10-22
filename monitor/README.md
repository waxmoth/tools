# Host Resource Monitor

A local page for CPU, memory, and NVIDIA GPU usage. Samples are kept in SQLite for 7 days.

## Usage

* Run it directly. This listens on `127.0.0.1` only.

```bash
python3 src/server.py
# python3 src/server.py 8780
```

* Or run it in Docker. The container reads the host `/proc`, `/sys`, and `nvidia-smi`, so the page shows the host.

```bash
docker compose up --build
```

Open [http://127.0.0.1:8780](http://127.0.0.1:8780). History is stored in `data/`.
