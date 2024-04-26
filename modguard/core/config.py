from typing import List, Dict

from pydantic import BaseModel


class Config(BaseModel):
    model_config = {"extra": "forbid"}


class ModuleConfig(Config):
    """
    Configuration for a single module within a project.
    """

    tags: List[str]
    strict: bool = False

    @classmethod
    def from_yml(cls, content: str) -> "ModuleConfig":
        # TODO: Mocking for now
        return cls(tags=["test"], strict=False)


class ScopeDependencyRules(Config):
    """
    Dependency rules for a particular scope.
    """

    depends_on: List[str]


class ProjectConfig(Config):
    """
    Configuration applied globally to a project.
    """

    tags: Dict[str, ScopeDependencyRules]
    ignore: List[str]
