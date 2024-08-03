from pathlib import Path

from setuptools import setup


# For this trick, I thank:
# https://stackoverflow.com/a/78056725

root: Path = Path(__file__).parent

if __name__ == "__main__":
    setup(
        install_requires=[
            f"myorg-pack-{letter} @ {(root.parent / f'pack-{letter}').as_uri()}"
            for letter in ["a", "b", "c", "d", "e", "f"]
        ],
    )
