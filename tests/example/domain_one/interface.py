from modguard import public


@public(allowlist=["example.domain_two"])
def domain_one_interface():
    ...


domain_one_var = "hello domain two"

public(domain_one_var, allowlist=["example.domain_two"])
