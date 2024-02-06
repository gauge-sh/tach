from modguard import public


@public(allowlist=["example.domain_two"])
def domain_one_interface():
    ...
