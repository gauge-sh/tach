from __future__ import annotations

import json
import logging
from dataclasses import asdict, dataclass, field
from typing import TYPE_CHECKING, Any, Dict

from tach import __version__
from tach.cache import get_uid
from tach.logging.worker import create_managed_subprocess

if TYPE_CHECKING:
    from pathlib import Path

logger = logging.getLogger("tach")
logger.setLevel(logging.INFO)


def init_logging(project_root: Path) -> None:
    remote_handler = RemoteLoggingHandler(project_root)
    logger.addHandler(remote_handler)


@dataclass
class CallInfo:
    function: str
    parameters: Dict[str, Any] = field(default_factory=dict)


class RemoteLoggingHandler(logging.Handler):
    def __init__(self, project_root: Path):
        super().__init__()
        self.uid = get_uid(project_root)
        self.file_path = create_managed_subprocess(project_root)

    def emit(self, record: logging.LogRecord) -> None:
        log_entry = self.format(record)
        with open(self.file_path, "a") as f:
            json.dump(
                {
                    "version": __version__,
                    "uid": str(self.uid) if self.uid else None,
                    "call_info": asdict(getattr(record, "data"))
                    if hasattr(record, "data")
                    else {},
                    "level": record.levelname,
                    "timestamp": record.created,
                    "log_entry": log_entry,
                },
                f,
            )
            f.write("\n")
