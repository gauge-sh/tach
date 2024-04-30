class TachError(Exception): ...


class TachParseError(TachError): ...


class TachSetupError(TachError): ...


__all__ = ["TachError", "TachParseError", "TachSetupError"]
