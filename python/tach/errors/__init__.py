from __future__ import annotations


class TachError(Exception): ...


class TachParseError(TachError): ...


class TachSetupError(TachError): ...


class TachCircularDependencyError(TachError):
    def __init__(self, cycles: list[list[str]]):
        self.cycles = cycles


__all__ = [
    "TachError",
    "TachParseError",
    "TachSetupError",
    "TachCircularDependencyError",
]
