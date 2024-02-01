# modguard
---
A Python tool to guard against incorrect usage of python modules.


### What is modguard?
Modguard enables you to wrap Python functions, classes, and modules to prevent unintended usage across your codebase.



### Installation
```bash
pip install modguard
```
[PyPi](#TODO)

### Usage
```python
# core/utils.py

@guard(deny=['core.utils'])
def retrieve_password(user_id: int) -> str:
    ...


def some_util() -> None:
    retrieve_password()
```
```bash
> # From the root of your project
> modguard .
1 error found
core/utils.py:L45-60 E001 Restricted usage of 'retrieve_password'
...
```
