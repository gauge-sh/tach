import example.domain_three.api
from ..domain_one.interface import domain_one_interface, domain_one_var
from ..domain_three import api

api.PublicForDomainTwo()
example.domain_three.api.PublicForDomainTwo()
domain_one_interface()
domain_two_var = domain_one_var
