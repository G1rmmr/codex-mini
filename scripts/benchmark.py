#!/usr/bin/env python3
"""Measure idle application overhead without submitting a model request.

Requires psutil. Existing desktop processes are observed, never restarted.
Only processes started by this script are terminated during cleanup.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import queue
import shutil
import statistics
import subprocess
import threading
import time

import psutil


def spawn(command, **kwargs):
    if os.name == "nt":
        kwargs["creationflags"] = subprocess.CREATE_NO_WINDOW
    return subprocess.Popen(command, **kwargs)


def family(process):
    try:
        return [process, *process.children(recursive=True)]
    except psutil.NoSuchProcess:
        return []


def cleanup(process):
    # Capture identities before terminating; never search by executable name.
    members = family(psutil.Process(process.pid)) if process.poll() is None else []
    for member in reversed(members):
        try:
            member.terminate()
        except psutil.NoSuchProcess:
            pass
    _, alive = psutil.wait_procs(members, timeout=3)
    for member in alive:
        try:
            member.kill()
        except psutil.NoSuchProcess:
            pass
    process.wait(timeout=5)


def cli_start(binary, cwd):
    started = time.perf_counter()
    process = spawn(
        [binary, "app-server", "--listen", "stdio://"],
        cwd=cwd, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, text=True, encoding="utf-8",
    )
    incoming = queue.Queue()

    def read():
        for line in process.stdout:
            try:
                incoming.put(json.loads(line))
            except ValueError:
                pass
        incoming.put(None)

    threading.Thread(target=read, daemon=True).start()

    def send(value):
        process.stdin.write(json.dumps(value) + "\n")
        process.stdin.flush()

    def response(identifier):
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            message = incoming.get(timeout=max(0.01, deadline - time.monotonic()))
            if message is None:
                raise RuntimeError("Codex app-server exited during initialization")
            if message.get("id") == identifier:
                if "error" in message:
                    raise RuntimeError("Codex metadata request failed")
                return message.get("result", {})
        raise TimeoutError("Codex metadata request timed out")

    try:
        send({"id": 1, "method": "initialize", "params": {
            "clientInfo": {"name": "codex_mini", "title": "Codex Mini", "version": "0.1.0"}}})
        response(1)
        send({"method": "initialized", "params": {}})
        send({"id": 2, "method": "account/read", "params": {"refreshToken": False}})
        response(2)
        cursor = None
        models = 0
        identifier = 3
        while True:
            params = {"limit": 50}
            if cursor:
                params["cursor"] = cursor
            send({"id": identifier, "method": "model/list", "params": params})
            result = response(identifier)
            models += len(result.get("data", []))
            cursor = result.get("nextCursor")
            if not cursor:
                break
            identifier += 1
        return process, {"startup_ms": (time.perf_counter() - started) * 1000, "models": models}
    except BaseException:
        cleanup(process)
        raise


def snapshot(processes):
    result = {}
    for process in processes:
        try:
            with process.oneshot():
                mem = process.memory_info()
                cpu = process.cpu_times()
                result[(process.pid, process.create_time())] = {
                    "cpu": cpu.user + cpu.system, "rss": mem.rss,
                    "private": getattr(mem, "private", None), "name": process.name(),
                }
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            continue
    return result


def summarize(samples):
    if not samples or not all(s["process_count"] for s in samples):
        raise RuntimeError("A measured process group disappeared")
    return {
        "rss_mib_median": statistics.median(s["rss_mib"] for s in samples),
        "rss_mib_peak": max(s["rss_mib"] for s in samples),
        "private_mib_median": statistics.median(s["private_mib"] for s in samples)
        if all(s["private_mib"] is not None for s in samples) else None,
        "cpu_one_core_pct_mean": sum(s["cpu_seconds"] for s in samples)
        / sum(s["interval_seconds"] for s in samples) * 100,
        "process_count_median": statistics.median(s["process_count"] for s in samples),
    }


def measure(groups, seconds):
    previous = {label: snapshot(getter()) for label, getter in groups.items()}
    samples = {label: [] for label in groups}
    started = previous_time = time.perf_counter()
    while time.perf_counter() - started < seconds:
        time.sleep(min(0.5, max(0, seconds - (time.perf_counter() - started))))
        now = time.perf_counter()
        elapsed = now - previous_time
        for label, getter in groups.items():
            current = snapshot(getter())
            old = previous[label]
            delta = sum(max(0, v["cpu"] - old[k]["cpu"]) for k, v in current.items() if k in old)
            private = [v["private"] for v in current.values()]
            samples[label].append({
                "elapsed_seconds": now - started, "interval_seconds": elapsed,
                "cpu_seconds": delta, "cpu_one_core_pct": delta / elapsed * 100,
                "rss_mib": sum(v["rss"] for v in current.values()) / 2**20,
                "private_mib": sum(private) / 2**20 if private and None not in private else None,
                "process_count": len(current), "pids": [k[0] for k in current],
            })
            previous[label] = current
        previous_time = now
    return {label: {"summary": summarize(rows), "samples": rows} for label, rows in samples.items()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mini", type=Path, required=True)
    parser.add_argument("--codex", default=shutil.which("codex"))
    parser.add_argument("--desktop-pid", type=int)
    parser.add_argument("--cwd", type=Path, default=Path.cwd())
    parser.add_argument("--output", type=Path, default=Path("dist/benchmark.json"))
    parser.add_argument("--trials", type=int, default=3)
    parser.add_argument("--seconds", type=float, default=15)
    parser.add_argument("--warmup", type=float, default=3)
    args = parser.parse_args()
    if not args.codex or args.trials < 1 or args.seconds < 1 or args.warmup < 0:
        parser.error("codex, positive trials/seconds and nonnegative warmup are required")
    args.output = args.output.resolve()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    desktop = psutil.Process(args.desktop_pid) if args.desktop_pid else None
    desktop_exe = desktop.exe() if desktop else None
    sampler = psutil.Process()
    parent = sampler.parent()
    harness = parent if parent and parent.name().lower() in {
        "pwsh.exe", "powershell.exe", "cmd.exe", "bash", "zsh", "sh"
    } else sampler
    results = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "system": {"os": platform.platform(), "logical_cpus": psutil.cpu_count(),
                   "ram_gib": psutil.virtual_memory().total / 2**30},
        "codex_version": subprocess.check_output([args.codex, "--version"], text=True).strip(),
        "mini_exe_bytes": args.mini.stat().st_size,
        "mini_sha256": hashlib.sha256(args.mini.read_bytes()).hexdigest(),
        "desktop": {"pid": desktop.pid, "executable": desktop_exe} if desktop else None,
        "excluded_sampler_harness": {"pid": harness.pid, "name": harness.name()},
        "protocol": {"trials": args.trials, "sample_seconds": args.seconds, "warmup_seconds": args.warmup,
                     "sample_interval_seconds": 0.5, "model_requests": 0},
        "limitations": [
            "Codex CLI is measured in app-server stdio mode; terminal rendering is excluded.",
            "Mini is a fresh connected empty chat with the normal tray/journal services and software renderer.",
            "Desktop is an existing live session, not a fresh idle application; other chats/workers may contribute.",
            "chatgpt_ui includes only processes with the desktop executable; existing_total may include unrelated tools/previews and is not a clean app baseline.",
            "Working-set sums can double-count shared pages; private bytes are committed memory, not resident RAM.",
            "Startup is a warm launch through metadata readiness, not cold disk start or first rendered frame.",
            "No inference, streaming, scrolling, GPU memory, energy or low-end Linux measurements are included.",
        ],
        "trials": [],
    }
    for trial in range(args.trials):
        owned = []
        try:
            profile = args.output.parent / f"bench-profile-{time.time_ns()}"
            profile.mkdir()
            marker = profile / "ready.json"
            env = os.environ.copy()
            env["XDG_CONFIG_HOME"] = str(profile)
            started = time.perf_counter()
            mini = spawn([str(args.mini.resolve()), "--benchmark-idle", str(marker),
                          "--cwd", str(args.cwd.resolve()), "--codex", args.codex],
                         env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            owned.append(mini)
            deadline = time.monotonic() + 30
            while True:
                if mini.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError("Mini did not reach metadata readiness")
                try:
                    mini_ready = json.loads(marker.read_text(encoding="utf-8"))
                    break
                except (FileNotFoundError, json.JSONDecodeError):
                    pass
                time.sleep(0.05)
            mini_ready["external_startup_ms"] = (time.perf_counter() - started) * 1000
            if not mini_ready.get("backend_pid") or not mini_ready.get("models"):
                raise RuntimeError("Mini did not return a backend PID and model catalog")
            cli, cli_ready = cli_start(args.codex, str(args.cwd.resolve()))
            owned.append(cli)
            mini_root = psutil.Process(mini.pid)
            cli_root = psutil.Process(cli.pid)
            backend = psutil.Process(mini_ready["backend_pid"])
            groups = {
                "codex_cli_backend": lambda: family(cli_root),
                "mini_gui": lambda: [mini_root],
                "mini_backend": lambda: family(backend),
                "mini_total": lambda: family(mini_root),
            }
            if desktop:
                def desktop_family():
                    excluded = {p.pid for root in (mini_root, cli_root, harness) for p in family(root)}
                    return [p for p in family(desktop) if p.pid not in excluded]

                def desktop_ui():
                    items = []
                    for p in family(desktop):
                        try:
                            if p.exe() == desktop_exe:
                                items.append(p)
                        except (psutil.NoSuchProcess, psutil.AccessDenied):
                            pass
                    return items

                groups["chatgpt_ui"] = desktop_ui
                groups["chatgpt_existing_total"] = desktop_family
            time.sleep(args.warmup)
            print(f"Trial {trial + 1}: sampling {args.seconds:g}s", flush=True)
            data = measure(groups, args.seconds)
            results["trials"].append({"number": trial + 1, "mini_ready": mini_ready,
                                      "cli_ready": cli_ready, "groups": data})
            args.output.write_text(json.dumps(results, indent=2), encoding="utf-8")
            print(f"Trial {trial + 1} complete: Mini GUI "
                  f"{data['mini_gui']['summary']['rss_mib_median']:.1f} MiB, total "
                  f"{data['mini_total']['summary']['rss_mib_median']:.1f} MiB", flush=True)
        finally:
            for process in reversed(owned):
                cleanup(process)
    print(f"Saved {args.output}", flush=True)


if __name__ == "__main__":
    main()
