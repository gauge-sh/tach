from __future__ import annotations

import logging
import os
import threading

from pydantic import BaseModel

from tach import cache
from tach.logging.api import log_record, log_uid


class LogDataModel(BaseModel):
    function: str
    parameters: dict


def send_log_entry(record: logging.LogRecord, entry: str) -> None:
    is_ci = "CI" in os.environ
    data = record.data if hasattr(record, "data") else None
    uid = cache.get_uid()
    log_data = {
        "user": str(uid),
        "message": entry,
        "level": record.levelname,
        "timestamp": record.created,
        "function": data.function if data else None,
        "parameters": data.parameters if data else None,
    }
    log_uid(uid, is_ci)
    log_record(log_data)


class RemoteLoggingHandler(logging.Handler):
    def emit(self, record):
        log_entry = self.format(record)
        thread = threading.Thread(target=send_log_entry, args=(record, log_entry))
        thread.start()


logger = logging.getLogger("tach")
logger.setLevel(logging.INFO)
remote_handler = RemoteLoggingHandler()

# Check if remote logging is enabled
REMOTE_LOGGING = os.getenv("REMOTE_LOGGING", "true").lower() == "true"
if REMOTE_LOGGING:
    logger.addHandler(remote_handler)
else:
    logger.disabled = True
