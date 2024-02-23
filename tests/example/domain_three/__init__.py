import modguard
from ..domain_one.interface import (
    domain_one_interface,
    domain_one_var,
    domain_one_regex_interface,
)

modguard.Boundary()


# Usages
domain_one_interface()
domain_one_regex_interface()
local_var = domain_one_var
