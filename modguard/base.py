from collections.abc import Iterable
import inspect

def guard(allow: Iterable[str] = [], deny: Iterable[str] = [], ):
    def decorator(func: callable):
        def wrapper(*args, **kwargs):
            current_frame = inspect.currentframe()
            previous_frame = current_frame.f_back
            module = inspect.getmodule(previous_frame)
            module_name = module.__name__
            print(inspect.getmodule(current_frame).__name__)
            if allow and module_name not in allow:
                raise RuntimeError(f"{func.__name__} can only be invoked in {', '.join(allow)} modules.")
            if deny and module_name in deny:
                raise RuntimeError(f"{func.__name__} can not be invoked in {', '.join(deny)} modules.")
            return func(*args, **kwargs)
        return wrapper
    return decorator