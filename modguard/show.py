import yaml
from modguard.core.boundary import BoundaryTrie
from typing import Any, Dict

# This type hint only works on more recent versions
# result_dict: TypeAlias = dict[str, str | bool | 'result_dict']

def boundary_trie_to_dict(boundary_trie: BoundaryTrie) -> Dict[str, Any]:
    result: Dict[str, Any] = dict()
    for node in boundary_trie:
        path = node.full_path
        if path == "":
            path = "root"
        sections = path.split(".")
        pointer: Dict[str, Any] = result
        for section in sections:
            if section not in pointer:
                pointer[section] = dict()
            pointer = pointer[section]
        pointer["is_boundary"] = True

        for member in node.public_members.keys():
            pointer: Dict[str, Any] = result
            sections = member.split(".")
            for section in sections:
                if section not in pointer:
                    pointer[section] = dict()
                pointer = pointer[section]
            pointer["is_public"] = True

    return result


def show(boundary_trie: BoundaryTrie) -> str:
    dict_repr = boundary_trie_to_dict(boundary_trie)
    result = yaml.dump(dict_repr)
    print(result)
    return result
