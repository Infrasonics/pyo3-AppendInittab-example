import emb


def multiply(a,b):
    print("Num args from embedded: ", emb.numargs())
    print("Will compute", a, "times", b)
    c = 0
    for i in range(0, a):
        c = c + b
    return c

