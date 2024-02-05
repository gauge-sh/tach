from dataclasses import dataclass


def public(fn: callable = None, *, allowlist: list[str] = None):
    return fn


@dataclass
class PublicMember:
    name: str
    allowlist: list[str] = None
