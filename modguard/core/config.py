from typing import List, Dict

from pydantic import BaseModel


class ModuleConfig(BaseModel):
    """
    Configuration for a single module within a project.
    """

    tags: List[str]
    strict: bool = False

    @classmethod
    def from_yml(cls, content: str) -> "ModuleConfig":
        # TODO: Mocking for now
        return cls(scopes=["test"], strict=False)


class ScopeDependencyRules(BaseModel):
    """
    Dependency rules for a particular scope.
    """

    depends_on: List[str]


class ProjectConfig(BaseModel):
    """
    Configuration applied globally to a project.
    """

    tags: Dict[str, ScopeDependencyRules]
    ignore: List[str]
