# modguard
---
A Python tool to guard against incorrect usage of python modules.


### What is modguard?
Modguard enables you to wrap Python functions, classes, and modules to prevent unintended usage across your codebase.

This helps prevent other developers unintentionally mis-using code, which can lead to poor performance, security vulnerabilities, bugs, and more.

Modguard is incredibly lightweight, and has no impact on the runtime of your code. Instead, it's checks are performed through a lightweight CLI tool.

### Installation
```bash
pip install modguard
```
[PyPi](#TODO)

### Usage
Implement the `guard` decorator and specify either the `allow` or `deny` parameter.
```python
# core/utils.py

@guard(deny=['core.utils, public.api'])
def retrieve_password(user_id: int) -> str:
    ...

def some_util():
    ...
    retrieve_password()
    ...
```
```bash
> # From the root of your project
> guard .
1 error found.
core/utils.py:L45-60 E001 Restricted usage of 'retrieve_password'
```
Modguard works across imports within the scope of your project
```
# public/api.py

from core.utils import retrieve_password

def some_api():
    ...
    retrieve_password()
    ...
```
```bash
> # From the root of your project
> modguard .
2 errors found.
core/utils.py:L45-60 E001 Restricted usage of 'retrieve_password' in 'core.utils.some_util'
public/api.py:L45-60 E001 Restricted usage of 'retrieve_password' in 'public.api.some_api'
```

### Interface
```python
"""Decorator for protecting against unintended usage. Note that you may only specify `allow` or `deny`, but not both.
Parameters:
    allow (Iterable[str]): A list of str representations of python module paths that are allowed to implement the decorated object.

    deny (Iterable[str]): A list of str representations of python module paths that are not allowed to implement the decorated object 

Return:
    None
"""
@guard(allow: Iterable[str]= [], deny: Iterable[str])
...
```




### Examples



### Details
Modguard works by analyzing the abstract syntax tree of your codebase. It will only protect against usages that are within the scope of the cli runtime, which is why we suggest always running the tool from the root of your project.

### License
[GNU GPLv3](LICENSE)
