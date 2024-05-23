from __future__ import annotations

import json
import logging
import os
import threading

import requests


def send_log_entry(url, record: logging.LogRecord, entry: str) -> None:
    try:
        response = requests.post(
            url,
            data=json.dumps({"log": entry}),
            headers={"Content-Type": "application/json"},
        )
        response.raise_for_status()
    except Exception:
        # Optionally, handle exceptions (e.g., logging to a file)
        print(record, entry)
        # print(f"Failed to send log entry: {entry}: {e}")


class RemoteLoggingHandler(logging.Handler):
    def __init__(self, url):
        super().__init__()
        self.url = url

    def emit(self, record):
        log_entry = self.format(record)
        thread = threading.Thread(
            target=send_log_entry, args=(self.url, record, log_entry)
        )
        thread.start()


logger = logging.getLogger("tach")
logger.setLevel(logging.INFO)
url = "https://your-logging-server.com/logs"
remote_handler = RemoteLoggingHandler(url)

# Check if remote logging is enabled
REMOTE_LOGGING = os.getenv("REMOTE_LOGGING", "true").lower() == "true"
if REMOTE_LOGGING:
    logger.addHandler(remote_handler)
else:
    logger.disabled = True
