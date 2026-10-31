def glibc(seed, n):
    seed &= 0xffffffff
    if seed == 0:
        seed = 1
    w = seed - (1 << 32) if seed >= 1 << 31 else seed  # int32
    r = [w]
    for i in range(1, 31):
        w = r[i - 1]
        hi = int(w / 127773)  # C truncating division
        lo = w - hi * 127773
        w = 16807 * lo - 2836 * hi
        if w < 0:
            w += 2147483647
        r.append(w)
    r = [x & 0xffffffff for x in r]
    f, rp = 3, 0
    out = []
    for k in range(310 + n):
        r[f] = (r[f] + r[rp]) & 0xffffffff
        v = r[f] >> 1
        f = (f + 1) % 31
        rp = (rp + 1) % 31
        if k >= 310:
            out.append(v)
    return out

print(glibc(1, 4))
