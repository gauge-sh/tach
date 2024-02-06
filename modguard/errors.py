from modguard import public

public()


class ModguardError(Exception):
    ...


class ModguardParseError(ModguardError):
    ...


class ModguardSetupError(ModguardError):
    ...
