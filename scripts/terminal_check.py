#!/usr/bin/env python3
"""Real-terminal smoke tests and CPU measurement; Python standard library only."""
import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import termios
import time


class Session:
    def __init__(self, binary, *args):
        self.master, self.slave = pty.openpty()
        self.original = termios.tcgetattr(self.slave)
        self.output = bytearray()
        self.resize(110, 32)

        def attach_terminal():
            os.setsid()
            fcntl.ioctl(0, termios.TIOCSCTTY, 0)

        # The agent shell may set NO_COLOR=1. Test an actual color-capable terminal.
        environment = {key: value for key, value in os.environ.items() if key != "NO_COLOR"}
        self.process = subprocess.Popen(
            [str(binary), *args], stdin=self.slave, stdout=self.slave,
            stderr=self.slave, env={**environment, "TERM": "xterm-256color", "COLORTERM": "truecolor"},
            preexec_fn=attach_terminal,
        )

    def resize(self, width, height):
        fcntl.ioctl(self.master, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
        if hasattr(self, "process"):
            self.process.send_signal(signal.SIGWINCH)

    def pump(self, duration):
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            ready, _, _ = select.select([self.master], [], [], min(0.05, max(0, deadline - time.monotonic())))
            if ready:
                self.output.extend(os.read(self.master, 65536))

    def until(self, text, timeout=10):
        deadline = time.monotonic() + timeout
        while text not in self.output and time.monotonic() < deadline:
            self.pump(0.05)
            if self.process.poll() is not None:
                break
        assert text in self.output, f"missing {text!r}: {self.output[-3000:]!r}"

    def send(self, data):
        os.write(self.master, data)

    def finish(self):
        deadline = time.monotonic() + 5
        while self.process.poll() is None and time.monotonic() < deadline:
            self.pump(0.05)
        assert self.process.poll() == 0, f"unexpected exit: {self.process.poll()}, {self.output[-2000:]!r}"
        self.pump(0.05)
        assert b"\x1b[?1049l" in self.output, "alternate screen was not restored"
        # macOS revokes the slave after the controlling process exits. The master
        # still exposes the shared terminal mode, so restoration remains testable.
        assert termios.tcgetattr(self.master) == self.original, "terminal mode was not restored"

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait()
        os.close(self.master)
        os.close(self.slave)

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()


def smoke(binary):
    for argument in ["--help", "-h", "--version", "-V"]:
        result = subprocess.run([str(binary), argument], capture_output=True, timeout=10)
        assert result.returncode == 0 and b"osbeat" in result.stdout
    for arguments in [["--invalid"], ["--debug", "--help"], []]:
        result = subprocess.run([str(binary), *arguments], capture_output=True, timeout=10)
        assert result.returncode != 0 and result.stderr
        assert b"\x1b[?1049h" not in result.stdout, "entered raw mode without a terminal"
    with Session(binary) as terminal:
        terminal.until(b"MEMORY")
        assert b"\x1b[38;2;" in terminal.output, "truecolor palette was not emitted"
        terminal.pump(0.8)  # Let the CPU baseline warm up.
        terminal.send(b"d")
        terminal.until(b"SELF CPU")
        terminal.send(b"d")
        terminal.pump(0.1)
        terminal.send(b"z")  # Unbound input leaves the app running.
        for width, height in [(1, 1), (30, 8), (55, 18), (80, 24), (110, 32)]:
            terminal.resize(width, height)
            terminal.pump(0.12)
            assert terminal.process.poll() is None, "crashed during resize"
        terminal.send(b"q")
        terminal.finish()
    for mode in ["ctrl-c", "SIGINT", "SIGTERM", "SIGHUP"]:
        with Session(binary, "--debug") as terminal:
            terminal.until(b"SELF CPU")
            if mode == "ctrl-c":
                terminal.send(b"\x03")
            else:
                terminal.process.send_signal(getattr(signal, mode))
            terminal.finish()
    print("PTY: startup, diagnostics, resize, q, Ctrl-C, SIGINT/TERM/HUP, terminal restoration passed")


def cpu_seconds(pid):
    result = subprocess.run(["ps", "-o", "time=", "-p", str(pid)], capture_output=True, text=True, check=True)
    parts = result.stdout.strip().split(":")
    return sum(float(part) * (60 ** index) for index, part in enumerate(reversed(parts)))


def measure(binary, duration, diagnostics, capture=None):
    with Session(binary, *(["--debug"] if diagnostics else [])) as terminal:
        terminal.until(b"MEMORY")
        terminal.pump(3)
        baseline_cpu = cpu_seconds(terminal.process.pid)
        baseline_wall = time.monotonic()
        terminal.pump(duration)
        elapsed = time.monotonic() - baseline_wall
        consumed = cpu_seconds(terminal.process.pid) - baseline_cpu
        usage = consumed / elapsed * 100
        rss = subprocess.run(["ps", "-o", "rss=", "-p", str(terminal.process.pid)], capture_output=True, text=True, check=True)
        result = {"wall_seconds": round(elapsed, 2), "cpu_percent_one_core": round(usage, 3),
                  "rss_mib": round(int(rss.stdout.strip()) / 1024, 2), "diagnostics": diagnostics,
                  "terminal": "110x32 truecolor PTY drained continuously", "sample_ms": 500}
        if capture:
            Path(capture).write_bytes(terminal.output)
        terminal.send(b"q")
        terminal.finish()
        print(json.dumps(result, indent=2))
        assert usage < 2.0, f"CPU budget exceeded: {usage:.3f}%"


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/release/osbeat"))
    parser.add_argument("--measure", type=float, help="Measure release CPU for this many seconds (>=30)")
    parser.add_argument("--debug", action="store_true")
    parser.add_argument("--capture", type=Path, help="Save ANSI output before quitting")
    options = parser.parse_args()
    binary = options.binary.resolve()
    if options.measure is not None:
        if options.measure < 30:
            parser.error("measure for at least 30 seconds")
        measure(binary, options.measure, options.debug, options.capture)
    else:
        smoke(binary)
