# from __future__ import annotations

# from copy import copy
# from dataclasses import dataclass
# from pathlib import Path
# from typing import TYPE_CHECKING, Any, ClassVar, List, Set

# from pydantic import AfterValidator, BaseModel, Field, field_serializer
# from typing_extensions import Annotated, Literal, Self

# from tach.constants import DEFAULT_EXCLUDE_PATHS, ROOT_MODULE_SENTINEL_TAG

# if TYPE_CHECKING:
#     from pydantic.fields import FieldInfo


# class Config(BaseModel):
#     ALWAYS_DUMP_FIELDS: ClassVar[Set[str]] = set()
#     model_config = {"extra": "forbid"}

#     @classmethod
#     def with_derived_unset_fields(cls, data: dict[str, Any]) -> Self:
#         # Build an instance of the class from Rust's parsed data
#         # but tell Pydantic which fields should be treated as 'unset'
#         fields_to_include = cls.ALWAYS_DUMP_FIELDS.copy()
#         for field_name, field_info in cls.model_fields.items():
#             if field_name not in data:
#                 continue
#             value = data[field_name]
#             default = cls._get_field_default(field_info)

#             if value != default or field_name in cls.ALWAYS_DUMP_FIELDS:
#                 fields_to_include.add(field_name)

#         return cls.model_construct(_fields_set=fields_to_include, **data)

#     @classmethod
#     def _get_field_default(cls, field_info: FieldInfo) -> Any:
#         if field_info.default_factory:
#             return field_info.default_factory()
#         return field_info.default


# class ModuleConfig(Config):
#     """
#     Configuration for a single module in a Tach project.

#     Primarily responsible for declaring dependencies.
#     """

#     ALWAYS_DUMP_FIELDS: ClassVar[Set[str]] = {"path", "depends_on"}

#     path: str
#     depends_on: List[Dependency] = Field(default_factory=list)
#     strict: bool = False

#     @property
#     def mod_path(self) -> str:
#         if self.path == ROOT_MODULE_SENTINEL_TAG:
#             return "."
#         return self.path


# class Dependency(Config):
#     ALWAYS_DUMP_FIELDS: ClassVar[Set[str]] = {"path"}

#     path: str
#     deprecated: bool = False

#     model_config = {"frozen": True}


# def validate_root_path(path: str) -> str:
#     assert path == ROOT_MODULE_SENTINEL_TAG
#     return path


# class RootModuleConfig(ModuleConfig):
#     """
#     Special-case schema for the implicit root module configuration.
#     """

#     path: Annotated[str, AfterValidator(validate_root_path)] = ROOT_MODULE_SENTINEL_TAG


# @dataclass
# class UnusedDependencies:
#     path: str
#     dependencies: List[Dependency]


# class CacheConfig(Config):
#     """
#     Configuration affecting Tach's caching.

#     Responsible for configuring the cache backend, and adjusting the dependencies which should affect cache retrieval.
#     """

#     backend: Literal["disk"] = "disk"
#     file_dependencies: List[str] = Field(default_factory=list)
#     env_dependencies: List[str] = Field(default_factory=list)


# class ExternalDependencyConfig(Config):
#     """
#     Configuration affecting Tach's external dependency checking.
#     """

#     exclude: List[str] = Field(default_factory=list)


# class ProjectConfig(Config):
#     """
#     Central configuration object for a project using Tach.

#     Controls which modules are defined, their dependencies, as well as global tool-related configuration.
#     """

#     ALWAYS_DUMP_FIELDS: ClassVar[Set[str]] = {"modules", "exclude"}

#     modules: List[ModuleConfig] = Field(default_factory=list)
#     cache: CacheConfig = Field(default_factory=CacheConfig)
#     external: ExternalDependencyConfig = Field(default_factory=ExternalDependencyConfig)
#     exclude: List[str] = Field(default_factory=lambda: copy(DEFAULT_EXCLUDE_PATHS))
#     source_roots: List[Path] = Field(default_factory=lambda: [Path(".")])
#     exact: bool = False
#     disable_logging: bool = False
#     ignore_type_checking_imports: bool = True
#     forbid_circular_dependencies: bool = False
#     use_regex_matching: bool = True
