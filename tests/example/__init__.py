import modguard

# intentional import violations
from .domain_one.interface import domain_one_interface, domain_one_var
from .domain_three.api import PublicForDomainTwo
from .domain_four.subsystem import private_subsystem_call
from .domain_five.inner import private_fn

# OK import
import example.domain_four
from .domain_five import pub_fn

# modguard-ignore
from .domain_two.other import internal_api


modguard.Boundary()


# Usages
pub_fn()
private_fn()
domain_one_interface()
example_usage = domain_one_var
PublicForDomainTwo()
private_subsystem_call()
example.domain_four
internal_api()
