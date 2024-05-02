from typing import List, Optional, Any

from pydantic import BaseModel, Field


class Config(BaseModel):
    model_config = {"extra": "forbid"}


class PackageConfig(Config):
    """
    Configuration for a single package within a project.
    """

    tags: List[str]
    strict: bool = False


class TagDependencyRules(Config):
    """
    Dependency rules for a particular set of tags (typically one tag).
    """

    tag: str
    depends_on: List[str]


def is_deprecated_project_config(config: dict[str, Any]) -> bool:
    if not config:
        return False
    if "constraints" in config and not (
        set(config.keys()) - {"constraints", "exclude", "exclude_hidden_paths"}
    ):
        # This appears to be a project config object,
        # the deprecated version will have a dict of constraints
        return isinstance(config["constraints"], dict)
    return False


def flatten_deprecated_config(config: dict[str, Any]):
    config["constraints"] = [
        {"tag": key, **value} for key, value in config.get("constraints", {}).items()
    ]


class ProjectConfig(Config):
    """
    Configuration applied globally to a project.
    """

    constraints: List[TagDependencyRules] = Field(default_factory=list)
    exclude: Optional[List[str]] = Field(default_factory=lambda: ["tests", "docs"])
    exclude_hidden_paths: Optional[bool] = True

    def dependencies_for_tag(self, tag: str) -> list[str]:
        return next(
            (
                constraint.depends_on
                for constraint in self.constraints
                if constraint.tag == tag
            ),
            [],  # type: ignore
        )

    def add_dependencies_to_tag(self, tag: str, dependencies: list[str]):
        current_dependency_rules = next(
            (constraint for constraint in self.constraints if constraint.tag == tag),
            None,
        )
        if not current_dependency_rules:
            # No constraint exists for tag, just add the new dependencies
            self.constraints.append(
                TagDependencyRules(tag=tag, depends_on=dependencies)
            )
        else:
            # Constraints already exist, set the union of existing and new as dependencies
            new_dependencies = set(current_dependency_rules.depends_on) | set(
                dependencies
            )
            current_dependency_rules.depends_on = list(new_dependencies)

    @classmethod
    def factory(cls, config: dict[str, Any]) -> tuple[bool, "ProjectConfig"]:
        """
        Using this factory to catch deprecated config and flag it to the caller
        """
        if is_deprecated_project_config(config):
            flatten_deprecated_config(config)
            return True, ProjectConfig(**config)
        return False, ProjectConfig(**config)  # type: ignore
