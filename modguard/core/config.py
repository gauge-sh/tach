from typing import List, Dict, Optional

from pydantic import BaseModel, Field


class Config(BaseModel):
    model_config = {"extra": "forbid"}


class ModuleConfig(Config):
    """
    Configuration for a single module within a project.
    """

    tags: List[str]
    strict: bool = False


class ScopeDependencyRules(Config):
    """
    Dependency rules for a particular scope.
    """

    depends_on: List[str]


class ProjectConfig(Config):
    """
    Configuration applied globally to a project.
    """

    constraints: Dict[str, ScopeDependencyRules] = Field(default_factory=dict)
    ignore: Optional[List[str]] = Field(default_factory=list)
