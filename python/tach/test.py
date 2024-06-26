from __future__ import annotations

from pathlib import Path

from tach import filesystem as fs
from tach.core import ModuleConfig, ProjectConfig
from tach.errors import TachError
from tach.filesystem.git_ops import get_changed_files
from tach.parsing import build_module_tree


def build_module_consumer_map(modules: list[ModuleConfig]) -> dict[str, list[str]]:
    consumer_map: dict[str, list[str]] = {}
    for module in modules:
        for dependency in module.depends_on:
            if dependency in consumer_map:
                consumer_map[dependency].append(module.mod_path)
            else:
                consumer_map[dependency] = [module.mod_path]
    return consumer_map


def find_affected_modules(
    root_module_path: str,
    module_consumers: dict[str, list[str]],
    known_affected_modules: set[str],
) -> set[str]:
    if root_module_path not in module_consumers:
        return known_affected_modules
    for consumer in module_consumers[root_module_path]:
        # avoid recursing on modules we have already seen to prevent infinite cycles
        if consumer not in known_affected_modules:
            known_affected_modules.add(consumer)
            known_affected_modules |= find_affected_modules(
                consumer,
                module_consumers=module_consumers,
                known_affected_modules=known_affected_modules,
            )
    return known_affected_modules


def get_affected_modules(
    project_root: Path, project_config: ProjectConfig, changed_files: list[Path]
) -> set[str]:
    source_root = project_root / project_config.source_root

    module_validation_result = fs.validate_project_modules(
        source_root=source_root, modules=project_config.modules
    )
    module_consumers = build_module_consumer_map(project_config.modules)
    # TODO: log warning
    for module in module_validation_result.invalid_modules:
        print(f"Module '{module.path}' not found. It will be ignored.")

    module_tree = build_module_tree(
        source_root=source_root,
        modules=module_validation_result.valid_modules,
    )
    changed_module_paths = [
        fs.file_to_module_path(
            source_root=source_root, file_path=changed_file.resolve()
        )
        for changed_file in changed_files
        if source_root in changed_file.resolve().parents
    ]

    affected_modules: set[str] = set()
    for changed_mod_path in changed_module_paths:
        nearest_module = module_tree.find_nearest(changed_mod_path)
        if nearest_module is None:
            raise TachError(
                f"Could not find module containing path: {changed_mod_path}"
            )
        affected_modules.add(nearest_module.full_path)

    for module in list(affected_modules):
        find_affected_modules(
            module,
            module_consumers=module_consumers,
            known_affected_modules=affected_modules,
        )
    return affected_modules


def run_affected_tests(
    project_root: Path,
    project_config: ProjectConfig,
    head: str = "",
    base: str = "main",
):
    # These paths come from git output, which means they are relative to cwd
    changed_files = get_changed_files(project_root, head=head, base=base)
    return get_affected_modules(
        project_root, project_config, changed_files=changed_files
    )
