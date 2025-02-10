from module5 import something


class MyModel:
    field = ForeignKey("module3.content")
    field2 = ForeignKey("module3.anything")