#!/usr/bin/env python3
"""Helpers for the isolated Wayfire blur experiment (standard library only).

Commands:
  ipc METHOD [JSON]                 call a Wayfire IPC method, print the reply
  wait-view APP_ID [SECONDS]        print the id of the first mapped view
  view-output APP_ID [SECONDS]      print the output name of a mapped view
  output-id NAME                    print the IPC id of an output
  place ID OUTPUT_ID X Y W H        move and resize a toplevel
  sweep ID X0 Y W H DX STEPS DELAY  move a toplevel horizontally in steps
  measure PID IDLE ID X0 Y W H DX STEPS DELAY
                                    CPU use while idle and while a toplevel
                                    moves, plus RSS (JSON)
  png PPM PNG                       convert a binary PPM to PNG
  edge PPM X0 Y0 X1 Y1              RMS horizontal luminance step in a region
  report DIR                        compare blur-on and blur-off captures

The IPC socket is read from WAYFIRE_SOCKET.
"""
import json
import os
import socket
import struct
import sys
import time
import zlib


def ipc(method, data=None):
    path = os.environ["WAYFIRE_SOCKET"]
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.connect(path)
        payload = json.dumps({"method": method, "data": data or {}}).encode()
        client.sendall(struct.pack("<I", len(payload)) + payload)
        header = _read(client, 4)
        (length,) = struct.unpack("<I", header)
        return json.loads(_read(client, length))


def _read(client, length):
    chunks = b""
    while len(chunks) < length:
        chunk = client.recv(length - len(chunks))
        if not chunk:
            raise ConnectionError("Wayfire closed the IPC socket")
        chunks += chunk
    return chunks


def wait_view(app_id, seconds=10.0):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        for view in ipc("window-rules/list-views"):
            if view.get("app-id") == app_id and view.get("mapped"):
                return view["id"]
        time.sleep(0.1)
    raise TimeoutError(f"No mapped view with app-id {app_id}")


def view_output(app_id, seconds=5.0):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        for view in ipc("window-rules/list-views"):
            if view.get("app-id") == app_id and view.get("mapped"):
                return view["output-name"]
        time.sleep(0.1)
    raise TimeoutError(f"No mapped view with app-id {app_id}")


def output_id(name):
    for output in ipc("window-rules/list-outputs"):
        if output.get("name") == name:
            return output["id"]
    raise LookupError(f"No output named {name}")


def place(view, output, x, y, width, height):
    reply = ipc(
        "window-rules/configure-view",
        {
            "id": view,
            "output_id": output,
            "geometry": {"x": x, "y": y, "width": width, "height": height},
        },
    )
    if reply.get("result") != "ok":
        raise RuntimeError(reply)


def sweep(view, x0, y, width, height, dx, steps, delay):
    for step in range(steps):
        reply = ipc(
            "window-rules/configure-view",
            {
                "id": view,
                "geometry": {"x": x0 + dx * step, "y": y, "width": width, "height": height},
            },
        )
        if reply.get("result") != "ok":
            raise RuntimeError(reply)
        time.sleep(delay)


def usage(pid):
    with open(f"/proc/{pid}/stat", encoding="ascii") as stat:
        fields = stat.read().rsplit(")", 1)[1].split()
    ticks = int(fields[11]) + int(fields[12])
    rss = 0
    with open(f"/proc/{pid}/status", encoding="ascii") as status:
        for line in status:
            if line.startswith("VmRSS:"):
                rss = int(line.split()[1])
    return {"ticks": ticks, "hz": os.sysconf("SC_CLK_TCK"), "rss_kib": rss}


def measure(pid, idle, view, x0, y, width, height, dx, steps, delay):
    def cpu_percent(before, after, seconds):
        return round(100.0 * (after["ticks"] - before["ticks"]) / before["hz"] / seconds, 1)

    start = usage(pid)
    time.sleep(idle)
    idle_end = usage(pid)
    began = time.monotonic()
    sweep(view, x0, y, width, height, dx, steps, delay)
    elapsed = time.monotonic() - began
    end = usage(pid)
    return {
        "idle_seconds": idle,
        "idle_cpu_percent": cpu_percent(start, idle_end, idle),
        "motion_steps": steps,
        "motion_seconds": round(elapsed, 2),
        "motion_cpu_percent": cpu_percent(idle_end, end, elapsed),
        "rss_kib": end["rss_kib"],
    }


def read_ppm(path):
    with open(path, "rb") as image:
        data = image.read()
    parts = []
    position = 0
    while len(parts) < 4:
        while data[position : position + 1].isspace():
            position += 1
        if data[position : position + 1] == b"#":
            position = data.index(b"\n", position) + 1
            continue
        end = position
        while not data[end : end + 1].isspace():
            end += 1
        parts.append(data[position:end])
        position = end
    if parts[0] != b"P6" or parts[3] != b"255":
        raise ValueError(f"{path} is not an 8-bit binary PPM")
    width, height = int(parts[1]), int(parts[2])
    pixels = data[position + 1 : position + 1 + width * height * 3]
    return width, height, pixels


def write_png(path, width, height, pixels):
    rows = b"".join(
        b"\x00" + pixels[row * width * 3 : (row + 1) * width * 3] for row in range(height)
    )

    def chunk(kind, body):
        return (
            struct.pack(">I", len(body))
            + kind
            + body
            + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)
        )

    with open(path, "wb") as image:
        image.write(b"\x89PNG\r\n\x1a\n")
        image.write(chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)))
        image.write(chunk(b"IDAT", zlib.compress(rows, 6)))
        image.write(chunk(b"IEND", b""))


def edge(path, x0, y0, x1, y1):
    """Root mean square luminance difference between horizontal neighbours.

    Sharp stripes behind a translucent surface give a high value; a blurred
    background spreads each step over many pixels and gives a low one. The
    mean absolute difference would barely change for wide stripes, because
    blur keeps their total variation. Both captures contain the same surface
    content, so the difference comes from the background.
    """
    width, height, pixels = read_ppm(path)
    if not (0 <= x0 < x1 <= width and 0 <= y0 < y1 <= height):
        raise ValueError("region outside the image")
    total = 0.0
    count = 0
    for y in range(y0, y1):
        previous = None
        for x in range(x0, x1):
            i = (y * width + x) * 3
            luminance = 0.2126 * pixels[i] + 0.7152 * pixels[i + 1] + 0.0722 * pixels[i + 2]
            if previous is not None:
                total += (luminance - previous) ** 2
                count += 1
            previous = luminance
    return round((total / count) ** 0.5, 2)


# Regions on the scale 1 output (1280x720) that hold no surface text or
# icons, so they mostly show what lies behind the translucent material.
REGIONS = {
    "panel": ("panel", (430, 686, 850, 706)),
    "window-behind-panel": ("behind-1", (430, 686, 850, 706)),
    "menu": ("menu", (262, 290, 372, 600)),
    "quick-settings": ("quick-settings", (930, 532, 1250, 546)),
    "sidebar": ("sidebar", (906, 400, 1262, 470)),
    "panel-scale-2": ("scale2", (300, 700, 1000, 712)),
}


def report(directory):
    results = {}
    for name, (capture, region) in REGIONS.items():
        row = {}
        for state in ("on", "off"):
            path = os.path.join(directory, f"{capture}-{state}.ppm")
            row[state] = edge(path, *region) if os.path.exists(path) else None
        results[name] = {"region": region, **row}
    for state in ("on", "off"):
        path = os.path.join(directory, f"usage-{state}.json")
        if os.path.exists(path):
            with open(path, encoding="utf-8") as usage_file:
                results[f"usage-{state}"] = json.load(usage_file)
    return results


def main(argv):
    command, args = argv[1], argv[2:]
    if command == "ipc":
        print(json.dumps(ipc(args[0], json.loads(args[1]) if len(args) > 1 else None), indent=1))
    elif command == "wait-view":
        print(wait_view(args[0], float(args[1]) if len(args) > 1 else 10.0))
    elif command == "view-output":
        print(view_output(args[0], float(args[1]) if len(args) > 1 else 5.0))
    elif command == "output-id":
        print(output_id(args[0]))
    elif command == "place":
        place(*map(int, args))
    elif command == "sweep":
        view, x0, y, width, height, dx, steps = map(int, args[:7])
        sweep(view, x0, y, width, height, dx, steps, float(args[7]))
    elif command == "measure":
        pid, idle, view, x0, y, width, height, dx, steps = map(int, args[:9])
        result = measure(pid, idle, view, x0, y, width, height, dx, steps, float(args[9]))
        print(json.dumps(result, indent=1))
    elif command == "png":
        write_png(args[1], *read_ppm(args[0]))
    elif command == "edge":
        print(edge(args[0], *map(int, args[1:5])))
    elif command == "report":
        print(json.dumps(report(args[0]), indent=1))
    else:
        raise SystemExit(f"unknown command {command}")


if __name__ == "__main__":
    main(sys.argv)
