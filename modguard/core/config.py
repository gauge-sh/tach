from typing import List

from pydantic import BaseModel, Field


class ModuleConfig(BaseModel):
    """
    Configuration for a single module within a project.
    """
    scopes: List[str] = Field(default_factory=list)
    strict: bool = False


class ScopeDependencyRules(BaseModel):
    """
    Dependency rules for a particular scope.
    """
    scope: str
    depends_on: List[str] = Field(default_factory=list)


class ProjectConfig(BaseModel):
    """
    Configuration applied globally to a project.
    """
    dependency_rules: List[ScopeDependencyRules] = Field(default_factory=list)
