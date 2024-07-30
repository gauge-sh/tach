from __future__ import annotations


class TachError(Exception): ...


class TachParseError(TachError): ...


class TachSetupError(TachError): ...


class TachCircularDependencyError(TachError):
    def __init__(self, module_paths: list[str]):
        self.module_paths = module_paths


__all__ = [
    "TachError",
    "TachParseError",
    "TachSetupError",
    "TachCircularDependencyError",
]
