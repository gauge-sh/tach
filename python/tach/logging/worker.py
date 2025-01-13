from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from http.client import HTTPSConnection
from pathlib import Path
from typing import Any
from urllib import error, parse, request

LOGGING_URL = "https://vmilasesnyvpalekembc.supabase.co"
PUBLIC_ANON_CLIENT_KEY = (
    "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InZtaWxhc2Vzbnl2cGFsZWtlbWJjIiwicm9"
    "sZSI6ImFub24iLCJpYXQiOjE3MTY0OTEwOTMsImV4cCI6MjAzMjA2NzA5M30.ndk9sUAmMJ5oNenDmLw35uT0s_d6c56Hk_PL5BucrOc"
)


def update_latest_version(project_root: Path) -> None:
    url = "https://pypi.org/pypi/tach/json"
    try:
        with request.urlopen(url, timeout=1) as response:
            if response.status == 200:
                data = response.read().decode()
                json_data = json.loads(data)
                latest_version = json_data["info"]["version"]
            else:
                return
    except (error.URLError, KeyError):
        return
    (project_root / ".tach" / ".latest-version").write_text(latest_version)


def log_request(url: str, data: dict[str, Any]) -> None:
    headers = {
        "Content-Type": "application/json",
        "apikey": PUBLIC_ANON_CLIENT_KEY,
        "authorization": f"Bearer {PUBLIC_ANON_CLIENT_KEY}",
    }
    json_data = json.dumps(data)
    full_url = f"{LOGGING_URL}/{url}"
    conn = None
    try:
        url_parts: parse.ParseResult = parse.urlparse(full_url)
        conn = HTTPSConnection(url_parts.netloc, timeout=1)
        conn.request("POST", full_url, body=json_data, headers=headers)
        conn.getresponse()
    except Exception:  # noqa
        pass
    finally:
        if conn is not None:
            conn.close()


def log_uid(uid: str, is_ci: bool, is_gauge: bool) -> None:
    log_request(
        url="rest/v1/User", data={"id": uid, "is_ci": is_ci, "is_gauge": is_gauge}
    )


def log_record(record_data: dict[str, Any]) -> None:
    log_request(url="rest/v1/LogRecord", data=record_data)


def send_log_entry(
    version: str,
    uid: str | None,
    function: str,
    parameters: dict[str, Any],
    level: str,
    timestamp: float,
    entry: str,
) -> None:
    is_ci = "CI" in os.environ
    is_gauge = "IS_GAUGE" in os.environ
    log_data: dict[str, Any] = {
        "user": str(uid) if uid else None,
        "message": entry,
        "level": level,
        "timestamp": timestamp,
        "function": function,
        "parameters": parameters,
        "version": version,
    }
    if uid:
        log_uid(uid=uid, is_ci=is_ci, is_gauge=is_gauge)
    log_record(log_data)


def process_message(message: dict[str, Any]) -> None:
    version = message["version"]
    uid = message["uid"]
    function = message["call_info"]["function"]
    parameters = message["call_info"]["parameters"]
    level = message["level"]
    timestamp = message["timestamp"]
    entry = message["log_entry"]
    send_log_entry(version, uid, function, parameters, level, timestamp, entry)


def subprocess_worker(fifo_path: Path, timeout: int = 5) -> None:
    try:
        with open(fifo_path) as pipe:
            last_message_time = time.time()
            while True:
                try:
                    line = pipe.readline()
                    if line:
                        message = json.loads(line)
                        process_message(message)
                        last_message_time = time.time()
                except json.JSONDecodeError:
                    continue
                except Exception:
                    pass

                time.sleep(0.1)
                if time.time() - last_message_time > timeout:
                    break

    except Exception:
        pass
    finally:
        if fifo_path.exists():
            fifo_path.unlink()


def create_managed_subprocess(project_root: Path, timeout: int = 5) -> Path:
    """
    Launches the worker as a completely separate process using subprocess.Popen.
    Returns the path to the named pipe for message passing.
    """
    tach_dir = project_root / ".tach"
    tach_dir.mkdir(parents=True, exist_ok=True)

    fifo_path = tach_dir / "log_pipe"
    if fifo_path.exists():
        fifo_path.unlink()
    os.mkfifo(fifo_path)

    worker_script = Path(__file__).resolve()
    subprocess.Popen(
        [sys.executable, str(worker_script), "--worker", str(fifo_path), str(timeout)],
        start_new_session=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    return fifo_path


# This is the entrypoint from subprocess.Popen in create_managed_subprocess
if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--worker":
        fifo_path = Path(sys.argv[2])
        timeout = int(sys.argv[3])
        subprocess_worker(fifo_path, timeout)
