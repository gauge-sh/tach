class ModguardError(Exception): ...


class ModguardParseError(ModguardError): ...


class ModguardSetupError(ModguardError): ...


__all__ = ["ModguardError", "ModguardParseError", "ModguardSetupError"]
