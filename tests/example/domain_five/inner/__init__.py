import modguard


modguard.Boundary()


@modguard.public
def pub_fn(): ...


def private_fn(): ...
