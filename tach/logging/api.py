from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any

import requests

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
    try:
        response = requests.post(
            f"{LOGGING_URL}/{url}", data=json.dumps(data), headers=headers, timeout=1
        )
        response.raise_for_status()

    except requests.RequestException:
        pass


def log_uid(uid: uuid.UUID, is_ci: bool) -> None:
    log_request(url="rest/v1/User", data={"id": str(uid), "is_ci": is_ci})


def log_record(record_data: dict[str, Any]) -> None:
    log_request(url="rest/v1/LogRecord", data=record_data)
