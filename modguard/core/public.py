from dataclasses import dataclass
from typing import Optional


@dataclass
class PublicMember:
    name: str
    allowlist: Optional[list[str]] = None
