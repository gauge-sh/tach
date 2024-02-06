from dataclasses import dataclass


@dataclass
class PublicMember:
    name: str
    allowlist: list[str] = None
