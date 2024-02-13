import modguard


@modguard.public(allowlist=["example.domain_two"])
def domain_one_interface():
    ...


domain_one_var = "hello domain two"

modguard.public(domain_one_var, allowlist=["example.domain_two"])
