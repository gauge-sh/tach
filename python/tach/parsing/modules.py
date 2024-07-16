from __future__ import annotations

from typing import TYPE_CHECKING

from tach.core import ModuleTree
from tach.errors import TachCircularDependencyError
from tach.parsing import parse_interface_members

if TYPE_CHECKING:
    from pathlib import Path

    from tach.core import ModuleConfig


def find_duplicate_modules(modules: list[ModuleConfig]) -> list[str]:
    duplicate_module_paths: list[str] = []
    seen: set[str] = set()
    for module in modules:
        if module.path in seen:
            duplicate_module_paths.append(module.path)
            continue
        seen.add(module.path)
    return duplicate_module_paths


def find_cycle(
    module: ModuleConfig,
    visited: set[str],
    path: list[str],
    modules: list[ModuleConfig],
    all_cycles: set[tuple[str, ...]],
) -> bool:
    if module.path in visited:
        cycle_start_index = path.index(module.path)
        cycle = path[cycle_start_index:] + [module.path]
        all_cycles.add(tuple(cycle))
        return True
    visited.add(module.path)
    path.append(module.path)
    for dependency in module.depends_on:
        dep_module = next((mod for mod in modules if mod.path == dependency), None)
        if dep_module:
            find_cycle(dep_module, visited, path, modules, all_cycles)
    visited.remove(module.path)
    path.pop()
    return False


def find_modules_with_circular_dependencies(
    modules: list[ModuleConfig],
) -> list[list[str]]:
    all_cycles: set[tuple[str, ...]] = set()
    for module in modules:
        visited: set[str] = set()
        path: list[str] = list()
        find_cycle(module, visited, path, modules, all_cycles)
    return [list(cycle) for cycle in all_cycles]


def build_module_tree(
    source_root: Path, modules: list[ModuleConfig], forbid_circular_dependencies: bool
) -> ModuleTree:
    duplicate_modules = find_duplicate_modules(modules)
    if duplicate_modules:
        raise ValueError(
            f"Failed to build module tree. The following modules were defined more than once: {duplicate_modules}"
        )
    if forbid_circular_dependencies:
        cycles = find_modules_with_circular_dependencies(modules)
        if cycles:
            raise TachCircularDependencyError(cycles)
    tree = ModuleTree()
    for module in modules:
        tree.insert(
            config=module,
            path=module.mod_path,
            interface_members=parse_interface_members(
                source_root=source_root, module_path=module.path
            ),
        )
    return tree
