from __future__ import annotations

import os
import sys
from enum import Enum
from functools import lru_cache
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from pathlib import Path


def is_interactive() -> bool:
    return sys.stdout.isatty() and sys.stderr.isatty()


class BCOLORS:
    HEADER = "\033[95m"
    OKBLUE = "\033[94m"
    OKCYAN = "\033[96m"
    OKGREEN = "\033[92m"
    WARNING = "\033[93m"
    FAIL = "\033[91m"
    ENDC = "\033[0m"
    BOLD = "\033[1m"
    UNDERLINE = "\033[4m"


def colorize(text: str, color_start: str, color_end: str = BCOLORS.ENDC) -> str:
    if is_interactive():
        return f"{color_start}{text}{color_end}"
    return text


class TerminalEnvironment(Enum):
    UNKNOWN = 1
    JETBRAINS = 2
    VSCODE = 3


@lru_cache(maxsize=None)
def detect_environment() -> TerminalEnvironment:
    if "jetbrains" in os.environ.get("TERMINAL_EMULATOR", "").lower():
        return TerminalEnvironment.JETBRAINS
    elif "vscode" in os.environ.get("TERM_PROGRAM", "").lower():
        return TerminalEnvironment.VSCODE
    return TerminalEnvironment.UNKNOWN


def render_path_simple(file_path: Path, line: int | None = None) -> str:
    if line is not None:
        return f"{file_path}[L{line}]"
    return str(file_path)


def create_clickable_link(
    file_path: Path, display_path: Path | None = None, line: int | None = None
) -> str:
    if not is_interactive():
        return render_path_simple(file_path, line)

    terminal_env = detect_environment()
    abs_path = file_path.resolve()

    if terminal_env == TerminalEnvironment.JETBRAINS:
        link = f"file://{abs_path}:{line}" if line is not None else f"file://{abs_path}"
    elif terminal_env == TerminalEnvironment.VSCODE:
        link = (
            f"vscode://file/{abs_path}:{line}"
            if line is not None
            else f"vscode://file/{abs_path}"
        )
    else:
        # For generic terminals, use a standard file link
        link = f"file://{abs_path}"

    # ANSI escape codes for clickable link
    if line:
        # Show the line number if we have it
        display_file_path = f"{display_path or file_path}[L{line}]"
    else:
        display_file_path = str(display_path) if display_path else str(file_path)
    clickable_link = f"\033]8;;{link}\033\\{display_file_path}\033]8;;\033\\"
    return clickable_link
