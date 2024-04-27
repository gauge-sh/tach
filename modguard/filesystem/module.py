import os
from typing import Optional

from modguard.constants import MODULE_FILE_NAME


def validate_module_config(root: str = ".") -> Optional[str]:
    file_path = os.path.join(root, f"{MODULE_FILE_NAME}.yml")
    if os.path.exists(file_path):
        return file_path
    file_path = os.path.join(root, f"{MODULE_FILE_NAME}.yaml")
    if os.path.exists(file_path):
        return file_path
    return


def validate_path(path: str) -> None:
    if not os.path.exists(path):
        raise FileNotFoundError()
    if os.path.isdir(path) and (
        os.path.exists(os.path.join(path, f"{MODULE_FILE_NAME}.yml"))
        or os.path.exists(os.path.join(path, f"{MODULE_FILE_NAME}.yaml"))
        or not os.path.exists(os.path.join(path, "__init__.py"))
    ):
        # TODO validate it's a python file
        raise ValueError()


def build_module(path: str, tags: list[str]) -> None:
    if os.path.isdir(path):
        with open(f"{path}/{MODULE_FILE_NAME}.yml", "w") as f:
            f.write(f"tags: [{','.join(tags)}]\n")
            # TODO should we write this into your modguard.yml as a set of minimum deps?
            return
    else:
        dirname = path.replace(".py", "")
        os.mkdir(dirname)
        with open(path, "r") as original_file:
            with open(f"{dirname}/main.py", "w") as new_file:
                new_file.write(original_file.read())
                # TODO write init.py, validate existing folder with same name doesn't already exist, write import, write module.yml
        os.remove(path)
