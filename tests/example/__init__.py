from modguard import Boundary

# intentional import violations
from .domain_one.interface import domain_one_interface, domain_one_var
from .domain_three.api import PublicForDomainTwo
from .domain_four.subsystem import private_subsystem_call

# OK import
import example.domain_four

# modguard-ignore
from .domain_two.other import internal_api


Boundary()


# Usages
domain_one_interface()
example_usage = domain_one_var
PublicForDomainTwo()
private_subsystem_call()
example.domain_four
internal_api()
