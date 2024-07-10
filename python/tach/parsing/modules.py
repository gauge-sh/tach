from __future__ import annotations

from typing import TYPE_CHECKING

from tach.core import ModuleTree
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

def find_modules_with_circular_dependencies(modules: list[ModuleConfig]) -> list[str]:
    def find_cycle(module: ModuleConfig, visited: set[str], path: list[str], modules: list[ModuleConfig]) -> list[str]:
        if module.path in visited:
            cycle_start_index = path.index(module.path)
            cycle = path[cycle_start_index:] + [module.path]
            return [f"{cycle[i]}: {cycle[i + 1]}" for i in range(len(cycle) - 1)]
        
        visited.add(module.path)
        path.append(module.path)
        
        for dependency in module.depends_on:
            dep_module = next((mod for mod in modules if mod.path == dependency), None)
            if dep_module:
                cycle = find_cycle(dep_module, visited, path, modules)
                if cycle:
                    return cycle

        visited.remove(module.path)
        path.pop()
        
        return []

    modules_with_cycles = set()
    
    for module in modules:
        cycle = find_cycle(module, set(), [], modules)
        if cycle:
            modules_with_cycles.update(cycle)
    
    return modules_with_cycles


def build_module_tree(source_root: Path, modules: list[ModuleConfig], forbid_circular_dependencies: bool) -> ModuleTree:
    duplicate_modules = find_duplicate_modules(modules)
    if duplicate_modules:
        raise ValueError(
            f"Failed to build module tree. The following modules were defined more than once: {duplicate_modules}"
        )
    if forbid_circular_dependencies:
        print("hmm", forbid_circular_dependencies)
        modules_with_cycles = find_modules_with_circular_dependencies(modules)
        if modules_with_cycles:
            raise ValueError(
                "Failed to build module tree. The following modules have circular dependencies\n" + 
                "\n".join(f"{cycle.split(': ')[0]}\t->\t{cycle.split(': ')[1]}" for cycle in modules_with_cycles)
            )
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
