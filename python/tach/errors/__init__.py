from __future__ import annotations
from tach.colors import BCOLORS

class TachError(Exception): ...


class TachParseError(TachError): ...


class TachSetupError(TachError): ...


class TachCircularDependencyError(TachError):
    def __init__(self, cycles: list[list[str]]):
        self.cycles = cycles
        super().__init__(f"❌ {BCOLORS.FAIL}Circular dependencies detected!\n\n" + "\n".join(
            f"{' -> '.join(cycle)}" for cycle in cycles
        ) + f"\n\n{BCOLORS.WARNING}Please resolve circular dependencies to continue.\n\nRemove 'forbid_circular_dependencies' from 'tach.yml' to allow circular dependencies.{BCOLORS.ENDC}")
        

__all__ = ["TachError", "TachParseError", "TachSetupError", "TachCircularDependencyError"]
