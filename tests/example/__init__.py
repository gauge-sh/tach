from modguard import Boundary

# intentional import violations
from .domain_one.interface import domain_one_interface
from .domain_three.api import public_for_domain_two
from .domain_four.subsystem import private_subsystem_call

# OK import
import example.domain_four

# modguard-ignore
from .domain_two.other import internal_api

Boundary()
