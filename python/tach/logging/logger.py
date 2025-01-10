from __future__ import annotations

import logging
import multiprocessing
import os
import sys
import threading
from dataclasses import dataclass, field
from typing import Any, Dict

from tach import __version__, cache
from tach.logging.api import log_record, log_uid
from tach.parsing import parse_project_config


@dataclass
class LogDataModel:
    function: str
    parameters: Dict[str, Any] = field(default_factory=dict)


def send_log_entry(record: logging.LogRecord, entry: str) -> None:
    is_ci = "CI" in os.environ
    is_gauge = "IS_GAUGE" in os.environ
    data: LogDataModel | None = getattr(record, "data", None)
    uid = cache.get_uid()
    version = __version__
    log_data: dict[str, Any] = {
        "user": str(uid) if uid else None,
        "message": entry,
        "level": record.levelname,
        "timestamp": record.created,
        "function": data.function if data else None,
        "parameters": data.parameters if data else None,
        "version": version,
    }
    if uid is not None:
        log_uid(uid=uid, is_ci=is_ci, is_gauge=is_gauge)
    log_record(record_data=log_data)
    cache.update_latest_version()


def handle_log_entry(record: logging.LogRecord, entry: str) -> None:
    with open(os.devnull, "w") as devnull:
        sys.stdout = devnull
        sys.stderr = devnull

        done = False

        def timeout_handler():
            nonlocal done
            if not done:
                os._exit(1)  # pyright: ignore

        # Start timeout timer
        timer = threading.Timer(5.0, timeout_handler)
        timer.start()

        try:
            send_log_entry(record=record, entry=entry)
        except Exception:  # noqa
            pass
        finally:
            done = True
            timer.cancel()


def spawn_log_entry(record: logging.LogRecord, entry: str) -> None:
    process = multiprocessing.Process(
        target=handle_log_entry, args=(record, entry), daemon=False
    )
    process.start()
    os._exit(0)  # pyright: ignore


class RemoteLoggingHandler(logging.Handler):
    def __init__(self):
        super().__init__()
        try:
            multiprocessing.set_start_method("spawn")
        except RuntimeError:
            # Method was already set, ignore
            pass

    def emit(self, record: logging.LogRecord) -> None:
        log_entry = self.format(record)
        # Ensure logs are nonblocking to main process
        process = multiprocessing.Process(
            target=spawn_log_entry,
            args=(record, log_entry),
        )
        process.start()


logger = logging.getLogger("tach")
logger.setLevel(logging.INFO)
remote_handler = RemoteLoggingHandler()

# Check if logging is enabled
disable_logging = False
project_config = parse_project_config()
if project_config:
    disable_logging = project_config.disable_logging
if not disable_logging:
    logger.addHandler(remote_handler)
else:
    logger.disabled = True
