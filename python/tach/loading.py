from __future__ import annotations

import itertools
import queue
import sys
import threading
import time

SPINNER_CHARS = ".oOo"
spinner_started = False
SPINNER_CHAR_FPS = 3
SPINNER_CHAR_TIME_DELAY = 1 / SPINNER_CHAR_FPS
SPINNER_DELAY = 0.35


end_signal_queue: queue.Queue[bool] = queue.Queue()
confirm_end_signal_queue: queue.Queue[bool] = queue.Queue()


def spinner(label: str = ""):
    time.sleep(SPINNER_DELAY)
    written = False
    for spinner_char in itertools.cycle(SPINNER_CHARS):
        line = f"{spinner_char} {label}" if label else spinner_char
        try:
            end_signal_queue.get(timeout=SPINNER_CHAR_TIME_DELAY)
            if written:
                sys.stdout.write("\b" * len(line))
                sys.stdout.flush()
            confirm_end_signal_queue.put_nowait(True)
            return
        except queue.Empty:
            pass
        sys.stdout.write(line)
        sys.stdout.flush()
        written = True
        sys.stdout.write("\b" * len(line))


def stop_spinner():
    global spinner_started
    if spinner_started:
        end_signal_queue.put_nowait(True)
        try:
            confirm_end_signal_queue.get(timeout=SPINNER_CHAR_TIME_DELAY + 0.2)
        except queue.Empty:
            pass
        spinner_started = False


def start_spinner(label: str = ""):
    global spinner_started
    if not spinner_started:
        threading.Thread(target=spinner, kwargs={"label": label}, daemon=True).start()
        spinner_started = True
