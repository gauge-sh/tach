from collections.abc import Iterable
import inspect

def guard(allow: Iterable[str] = [], deny: Iterable[str] = [], ):
    def decorator(func: callable):
        def wrapper(*args, **kwargs):
            current_frame = inspect.currentframe()
            previous_frame = current_frame.f_back
            module = inspect.getmodule(previous_frame)
            # Can use module name
            module_name = module.__name__
            # Also have function name
            function_name = previous_frame.f_code.co_name
            full_name = '.'.join((module_name, function_name))
            print(full_name)
            if allow and function_name not in allow:
                raise RuntimeError(f"{func.__name__} can only be invoked in {', '.join(allow)} modules.")
            if deny and function_name in deny:
                raise RuntimeError(f"{func.__name__} can not be invoked in {', '.join(deny)} modules.")
            return func(*args, **kwargs)
        return wrapper
    return decorator