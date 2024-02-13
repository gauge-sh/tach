import threading
import sys
import itertools
import queue
import time

SPINNER_CHARS = ".oOo"
spinner_started = False


end_signal_queue: queue.Queue[bool] = queue.Queue()


def spinner():
    for spinner_char in itertools.cycle(SPINNER_CHARS):
        try:
            end_signal_queue.get(block=False)
            return
        except queue.Empty:
            pass
        sys.stdout.write(spinner_char)
        sys.stdout.flush()
        time.sleep(0.33)
        sys.stdout.write("\b")


def stop_spinner():
    global spinner_started
    if spinner_started:
        end_signal_queue.put_nowait(True)
        spinner_started = False
        sys.stdout.write("\b")
        sys.stdout.flush()


def start_spinner():
    global spinner_started
    if not spinner_started:
        threading.Thread(target=spinner, daemon=True).start()
        spinner_started = True
