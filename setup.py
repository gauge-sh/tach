from setuptools import setup, find_packages

VERSION = '0.0.1' 
DESCRIPTION = 'Guard against mis-use of python modules, classes, and functions'
LONG_DESCRIPTION = 'ModGuard provides utilities to enforce mis-use of python modules, classes, and functions. Simply use the decorator to declare which modules your class is allowed to run and not run in.'

setup(
        name="modguard", 
        version=VERSION,
        author="Caelean Barnes & Evan Doyle",
        author_email="caeleanb@gmail.com",
        description=DESCRIPTION,
        long_description=LONG_DESCRIPTION,
        packages=find_packages(),
        install_requires=[],
        keywords=['python', 'module', 'guard', 'enforcement', 'enforcer', 'decorator', 'subclass'],
        classifiers= [
            "Development Status :: 3 - Alpha",
        ]
)
