from __future__ import annotations

import json
from http.client import HTTPSConnection
from typing import TYPE_CHECKING, Any
from urllib import parse

if TYPE_CHECKING:
    import uuid

LOGGING_URL = "https://vmilasesnyvpalekembc.supabase.co"
PUBLIC_ANON_CLIENT_KEY = (
    "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InZtaWxhc2Vzbnl2cGFsZWtlbWJjIiwicm9"
    "sZSI6ImFub24iLCJpYXQiOjE3MTY0OTEwOTMsImV4cCI6MjAzMjA2NzA5M30.ndk9sUAmMJ5oNenDmLw35uT0s_d6c56Hk_PL5BucrOc"
)


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


def log_uid(uid: uuid.UUID, is_ci: bool, is_gauge: bool) -> None:
    log_request(
        url="rest/v1/User", data={"id": str(uid), "is_ci": is_ci, "is_gauge": is_gauge}
    )


def log_record(record_data: dict[str, Any]) -> None:
    log_request(url="rest/v1/LogRecord", data=record_data)
