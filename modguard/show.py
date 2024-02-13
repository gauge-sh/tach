import yaml
from modguard.colors import BCOLORS
from modguard.core.boundary import BoundaryTrie
from typing import Any, Dict, Tuple


# This type hint only works on more recent versions
# result_dict: TypeAlias = dict[str, str | bool | 'result_dict']


def boundary_trie_to_dict(boundary_trie: BoundaryTrie) -> Dict[str, Any]:
    result: Dict[str, Any] = dict()
    for node in boundary_trie:
        path = node.full_path
        if path == "":
            continue
        sections = path.split(".")
        current: Dict[str, Any] = result
        for section in sections:
            if section not in current:
                current[section] = dict()
            current = current[section]
        current["is_boundary"] = True

        for member in node.public_members.keys():
            current: Dict[str, Any] = result
            sections = member.split(".")
            for section in sections:
                if section not in current:
                    current[section] = dict()
                current = current[section]
            current["is_public"] = True

    return result


def dict_to_str(dict_repr: Dict[str, Any]) -> str:
    str_repr = ""

    def _recurs_build_string(str_repr: str, level: int, current: Dict[str, Any]) -> str:
        for k, v in current.items():
            if isinstance(v, dict):
                is_boundary = "is_boundary" in v.keys()
                is_public = "is_public" in v.keys()
                str_repr += BCOLORS.ENDC + BCOLORS.ENDC + "\n" + "  " * level
                if is_boundary:
                    str_repr += BCOLORS.BOLD + "[B]"
                if is_public:
                    str_repr += BCOLORS.OKGREEN + "[P]"
                str_repr += k
                next_dict: Dict[str, Any] = v
                str_repr = _recurs_build_string(str_repr, level + 1, next_dict)
        return str_repr

    return _recurs_build_string(str_repr, 0, dict_repr) + "\n"


def show(boundary_trie: BoundaryTrie, write_file: bool = False) -> Tuple[str, str]:
    dict_repr = boundary_trie_to_dict(boundary_trie)
    yaml_result = yaml.dump(dict_repr)
    pretty_str_result = dict_to_str(dict_repr)
    if write_file:
        with open("modguard.yaml", "w") as f:
            yaml.dump(dict_repr, f)
    return yaml_result, pretty_str_result
