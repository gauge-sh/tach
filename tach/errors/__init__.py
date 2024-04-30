class tachError(Exception): ...


class tachParseError(tachError): ...


class tachSetupError(tachError): ...


__all__ = ["tachError", "tachParseError", "tachSetupError"]
