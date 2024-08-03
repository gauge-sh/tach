from __future__ import annotations

from copy import copy
from dataclasses import dataclass
from pathlib import Path
from typing import List, Optional

from pydantic import AfterValidator, BaseModel, Field, field_serializer
from typing_extensions import Annotated, Literal

from tach.constants import DEFAULT_EXCLUDE_PATHS, ROOT_MODULE_SENTINEL_TAG


class Config(BaseModel):
    model_config = {"extra": "forbid"}


class ModuleConfig(Config):
    """
    Configuration for a single module in a Tach project.

    Primarily responsible for declaring dependencies.
    """

    path: str
    depends_on: List[Dependency] = Field(default_factory=list)
    strict: bool = False

    @property
    def mod_path(self) -> str:
        if self.path == ROOT_MODULE_SENTINEL_TAG:
            return "."
        return self.path


class Dependency(Config):
    path: str
    deprecated: bool = False

    model_config = {"frozen": True}


def validate_root_path(path: str) -> str:
    assert path == ROOT_MODULE_SENTINEL_TAG
    return path


class RootModuleConfig(ModuleConfig):
    """
    Special-case schema for the implicit root module configuration.
    """

    path: Annotated[str, AfterValidator(validate_root_path)] = ROOT_MODULE_SENTINEL_TAG


@dataclass
class UnusedDependencies:
    path: str
    dependencies: List[Dependency]


class CacheConfig(Config):
    """
    Configuration affecting Tach's caching.

    Responsible for configuring the cache backend, and adjusting the dependencies which should affect cache retrieval.
    """

    backend: Literal["local"] = "local"
    file_dependencies: List[str] = Field(default_factory=list)
    env_dependencies: List[str] = Field(default_factory=list)


class ExternalDependencyConfig(Config):
    """
    Configuration affecting Tach's external dependency checking.
    """

    exclude: List[str] = Field(default_factory=list)


class ProjectConfig(Config):
    """
    Central configuration object for a project using Tach.

    Controls which modules are defined, their dependencies, as well as global tool-related configuration.
    """

    modules: List[ModuleConfig] = Field(default_factory=list)
    cache: CacheConfig = Field(default_factory=CacheConfig)
    external: ExternalDependencyConfig = Field(default_factory=ExternalDependencyConfig)
    exclude: Optional[List[str]] = Field(
        default_factory=lambda: copy(DEFAULT_EXCLUDE_PATHS)
    )
    source_roots: List[Path] = Field(default_factory=lambda: [Path(".")])
    exact: bool = False
    disable_logging: bool = False
    ignore_type_checking_imports: bool = True
    forbid_circular_dependencies: bool = False

    @field_serializer("source_roots")
    def serialize_source_roots(self, source_roots: list[Path], _) -> List[str]:
        return list(map(str, source_roots))

    @property
    def module_paths(self) -> list[str]:
        return [module.path for module in self.modules]

    def set_modules(self, module_paths: list[str]) -> None:
        new_modules: list[ModuleConfig] = []
        new_module_paths = set(module_paths)
        original_modules_by_path = {
            module_config.path: module_config for module_config in self.modules
        }
        for new_module_path in new_module_paths:
            if new_module_path in original_modules_by_path:
                original_module = original_modules_by_path[new_module_path]
                original_module.depends_on = [
                    dep
                    for dep in original_module.depends_on
                    if dep.path in new_module_paths
                ]
                new_modules.append(original_module)
            else:
                new_modules.append(ModuleConfig(path=new_module_path))

        self.modules = new_modules

    def dependencies_for_module(self, module: str) -> list[Dependency]:
        return next(
            (mod.depends_on for mod in self.modules if mod.path == module),
            list(),  # type: ignore
        )

    def add_dependency_to_module(self, module: str, dependency: Dependency):
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
            new_dependencies = set(current_module_config.depends_on) | {dependency}  # pyright: ignore[reportUnhashable]
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
            own_module_dependency_paths = set(
                dep.path
                for dep in self.dependencies_for_module(module=module_config.path)
            )
            current_dependency_paths = set(dep.path for dep in module_config.depends_on)
            extra_dependency_paths = (
                current_dependency_paths - own_module_dependency_paths
            )
            if extra_dependency_paths:
                extra_dependencies = [
                    dep
                    for dep in module_config.depends_on
                    if dep.path in extra_dependency_paths
                ]
                all_unused_dependencies.append(
                    UnusedDependencies(
                        path=module_config.path,
                        dependencies=extra_dependencies,
                    )
                )

        return all_unused_dependencies
