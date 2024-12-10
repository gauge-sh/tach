from __future__ import annotations


class TachError(Exception): ...


class TachSetupError(TachError): ...


class TachClosedBetaError(TachError): ...


class TachCircularDependencyError(TachError):
    def __init__(self, dependencies: list[str]):
        self.dependencies = dependencies
        super().__init__("Circular dependency error")


class TachVisibilityError(TachError):
    def __init__(self, visibility_errors: list[tuple[str, str, list[str]]]):
        self.visibility_errors = visibility_errors
        super().__init__("Visibility error")
