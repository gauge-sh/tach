from __future__ import annotations

import sys


def start():
    try:
        from tach.cli import main

        main()
    except KeyboardInterrupt:
        print("Exiting...")
        sys.exit(1)


if __name__ == "__main__":
    start()

__all__ = ["start"]
