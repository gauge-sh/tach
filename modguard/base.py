import inspect

def guard(allow: iter[str] = [], deny: iter[str] = []):
    def decorator(func: callable):
        def wrapper(*args, **kwargs):
            module_name = inspect.getmodule(inspect.currentframe().f_back).__name__
            if allow and module_name not in allow:
                raise RuntimeError(f"{func.__name__} can only be invoked in {', '.join(allow)} modules.")
            if deny and module_name in deny:
                raise RuntimeError(f"{func.__name__} can not be invoked in {', '.join(deny)} modules.")
            return func(*args, **kwargs)
        return wrapper
    return decorator