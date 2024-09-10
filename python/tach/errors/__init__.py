from __future__ import annotations


class TachError(Exception): ...


class TachSetupError(TachError): ...


__all__ = [
    "TachError",
    "TachSetupError",
]
