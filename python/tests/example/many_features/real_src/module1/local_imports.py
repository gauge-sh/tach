def do_something():
    from . import api
    from .submodule import helper
    from ..module2 import service
    api.call()
    helper.help()
    service.serve() 