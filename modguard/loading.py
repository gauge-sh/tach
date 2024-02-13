import threading
import sys
import itertools
import queue
import time

SPINNER_CHARS = ".oOo"
spinner_started = False


end_signal_queue: queue.Queue[bool] = queue.Queue()


def spinner(label: str = ""):
    time.sleep(0.5)
    written = False
    for spinner_char in itertools.cycle(SPINNER_CHARS):
        line = f"{spinner_char} {label}" if label else spinner_char
        try:
            end_signal_queue.get(timeout=0.33)
            if written:
                sys.stdout.write("\b" * len(line))
                sys.stdout.flush()
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
        time.sleep(0.01)
        spinner_started = False


def start_spinner(label: str = ""):
    global spinner_started
    if not spinner_started:
        threading.Thread(target=spinner, kwargs={"label": label}, daemon=True).start()
        spinner_started = True
