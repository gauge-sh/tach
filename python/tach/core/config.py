from __future__ import annotations

from typing import Any, List, Optional

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
    if "exclude_hidden_paths" in config:
        return True
    if "constraints" in config and not (
        set(config.keys()) - {"constraints", "exclude"}
    ):
        # This appears to be a project config object,
        # the deprecated version will have a dict of constraints
        return isinstance(config["constraints"], dict)
    return False


def fix_deprecated_config(config: dict[str, Any]):
    if "constraints" in config and isinstance(config["constraints"], dict):
        config["constraints"] = [
            {"tag": key, **value}
            for key, value in config.get("constraints", {}).items()
        ]
    if "exclude_hidden_paths" in config:
        config.pop("exclude_hidden_paths")


class ProjectConfig(Config):
    """
    Configuration applied globally to a project.
    """

    constraints: List[TagDependencyRules] = Field(default_factory=list)
    exclude: Optional[List[str]] = Field(default_factory=lambda: ["tests", "docs"])
    exact: bool = False
    disable_logging: bool = False
    ignore_type_checking_imports: bool = False

    def dependencies_for_tag(self, tag: str) -> list[str]:
        return next(
            (
                constraint.depends_on
                for constraint in self.constraints
                if constraint.tag == tag
            ),
            [],  # type: ignore
        )

    def add_dependencies_to_tags(self, tags: list[str], dependencies: list[str]):
        for tag in tags:
            current_dependency_rules = next(
                (
                    constraint
                    for constraint in self.constraints
                    if constraint.tag == tag
                ),
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

    def find_extra_constraints(
        self, other_config: "ProjectConfig"
    ) -> list[TagDependencyRules]:
        extra_constraints: list[TagDependencyRules] = []
        base_constraint_tags = set(constraint.tag for constraint in self.constraints)
        for constraint in other_config.constraints:
            if constraint.tag not in base_constraint_tags:
                extra_constraints.append(constraint)
                continue
            base_constraint_dependencies = set(
                self.dependencies_for_tag(constraint.tag)
            )
            extra_dependencies = (
                set(other_config.dependencies_for_tag(constraint.tag))
                - base_constraint_dependencies
            )
            if extra_dependencies:
                extra_constraints.append(
                    TagDependencyRules(
                        tag=constraint.tag,
                        depends_on=list(extra_dependencies),
                    )
                )

        return extra_constraints

    @classmethod
    def factory(cls, config: dict[str, Any]) -> tuple[bool, "ProjectConfig"]:
        """
        Using this factory to catch deprecated config and flag it to the caller
        """
        if is_deprecated_project_config(config):
            fix_deprecated_config(config)
            return True, ProjectConfig(**config)
        return False, ProjectConfig(**config)  # type: ignore
