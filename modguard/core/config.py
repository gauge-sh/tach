from typing import List, Dict

from pydantic import BaseModel, Field


class ModuleConfig(BaseModel):
    """
    Configuration for a single module within a project.
    """

    tags: List[str] = Field(default_factory=list)
    strict: bool = False

    @classmethod
    def from_yml(cls, content: str) -> "ModuleConfig":
        # TODO: Mocking for now
        return cls(scopes=["test"], strict=False)


class ScopeDependencyRules(BaseModel):
    """
    Dependency rules for a particular scope.
    """

    depends_on: List[str] = Field(default_factory=list)


class ProjectConfig(BaseModel):
    """
    Configuration applied globally to a project.
    """

    tags: Dict[str, ScopeDependencyRules] = Field(default_factory=dict)
    ignore: List[str] = Field(default_factory=list)
