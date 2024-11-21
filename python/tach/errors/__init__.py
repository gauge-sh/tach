from __future__ import annotations


class TachError(Exception): ...


class TachSetupError(TachError): ...


class TachClosedBetaError(TachError): ...
