from helper import do_work

fn run():
    print("Running Mojo")
    do_work()

struct User:
    var name: String
    fn __init__(inout self, name: String):
        self.name = name
