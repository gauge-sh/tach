from __future__ import annotations

from typing import TYPE_CHECKING

import cowsay

if TYPE_CHECKING:
    from pathlib import Path

    from tach.extension import ProjectConfig


def ai_check(project_root: Path, project_config: ProjectConfig):
    deepseek_check()


def openai_check():
    cowsay.cow("You can't compete with me!")


def deepseek_check():
    cowsay.tux("GPUs go brrrr")
