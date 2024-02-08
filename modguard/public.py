from typing import TypeVar, Optional

T = TypeVar("T")


def __inner_public(obj: T) -> T:
    return obj


def public(obj: T = __inner_public, *, allowlist: Optional[list[str]] = None) -> T:
    return obj
