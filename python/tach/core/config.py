from __future__ import annotations

from dataclasses import dataclass

from pydantic import AfterValidator, BaseModel, Field
from typing_extensions import Annotated

from tach.constants import ROOT_MODULE_SENTINEL_TAG


class Config(BaseModel):
    model_config = {"extra": "forbid"}


class ModuleConfig(Config):
    """
    Configuration for a single module in a Tach project.

    Primarily responsible for declaring dependencies.
    """

    path: str
    depends_on: list[str] = Field(default_factory=list)
    strict: bool = False


def validate_root_path(path: str) -> str:
    assert path == ROOT_MODULE_SENTINEL_TAG
    return path


class RootModuleConfig(ModuleConfig):
    """
    Special-case schema for the implicit root package configuration.
    """

    path: Annotated[str, AfterValidator(validate_root_path)] = ROOT_MODULE_SENTINEL_TAG


@dataclass
class UnusedDependencies:
    path: str
    dependencies: list[str]


class ProjectConfig(Config):
    """
    Central configuration object for a project using Tach.

    Controls which modules are defined, their dependencies, as well as global tool-related configuration.
    """

    modules: list[ModuleConfig] = Field(default_factory=list, frozen=True)
    exclude: list[str] | None = Field(default_factory=lambda: ["tests", "docs"])
    exact: bool = False
    disable_logging: bool = False
    ignore_type_checking_imports: bool = False

    def dependencies_for_module(self, module: str) -> list[str]:
        return next(
            (module.depends_on for module in self.modules if module.path == module),
            [],  # type: ignore
        )

    def add_dependency_to_module(self, module: str, dependency: str):
        current_module_config = next(
            (
                module_config
                for module_config in self.modules
                if module_config.path == module
            ),
            None,
        )
        if not current_module_config:
            # No configuration exists for tag, add default config with this dependency
            self.modules.append(ModuleConfig(path=module, depends_on=[dependency]))
        else:
            # Config already exists, set the union of existing dependencies and new ones
            new_dependencies = set(current_module_config.depends_on) | {dependency}
            current_module_config.depends_on = list(new_dependencies)

    def compare_dependencies(
        self, other_config: ProjectConfig
    ) -> list[UnusedDependencies]:
        all_unused_dependencies: list[UnusedDependencies] = []
        own_module_paths = set(module.path for module in self.modules)
        for module_config in other_config.modules:
            if module_config.path not in own_module_paths:
                all_unused_dependencies.append(
                    UnusedDependencies(
                        path=module_config.path, dependencies=module_config.depends_on
                    )
                )
                continue
            own_module_dependencies = set(
                self.dependencies_for_module(module=module_config.path)
            )
            extra_dependencies = set(module_config.depends_on) - own_module_dependencies
            if extra_dependencies:
                all_unused_dependencies.append(
                    UnusedDependencies(
                        path=module_config.path,
                        dependencies=list(extra_dependencies),
                    )
                )

        return all_unused_dependencies
