import re, sys
PALETTE = [(69, 71, 90), (243, 139, 168), (166, 227, 161), (249, 226, 175),
           (137, 180, 250), (203, 166, 247), (148, 226, 213), (205, 214, 244)]
FG, DIM_FG = (205, 214, 244), (127, 132, 156)
fg, bg, bold, dim = None, None, False, False

def xterm(n):
    if n < 16:
        return PALETTE[n % 8]
    if n < 232:
        n -= 16
        steps = [0, 95, 135, 175, 215, 255]
        return (steps[n // 36], steps[n // 6 % 6], steps[n % 6])
    v = 8 + (n - 232) * 10
    return (v, v, v)

def style():
    r, g, b = fg if fg else (DIM_FG if dim else FG)
    if dim and fg:
        r, g, b = [(c * 2 + 60) // 3 for c in (r, g, b)]
    out = f"\033[0;{'1;' if bold else ''}38;2;{r};{g};{b}m"
    if bg:
        out += "\033[48;2;%d;%d;%dm" % bg
    return out

def color(codes, i):
    if codes[i + 1] == 5:
        return xterm(codes[i + 2]), i + 2
    if codes[i + 1] == 2:
        return tuple(codes[i + 2:i + 5]), i + 4
    return None, i

def apply(text):
    global fg, bg, bold, dim
    codes = [int(c) for c in text.split(';') if c] or [0]
    i = 0
    while i < len(codes):
        c = codes[i]
        if c == 0: fg, bg, bold, dim = None, None, False, False
        elif c == 1: bold = True
        elif c == 2: dim = True
        elif c == 22: bold = dim = False
        elif c == 39: fg = None
        elif c == 49: bg = None
        elif 30 <= c <= 37: fg = PALETTE[c - 30]
        elif 90 <= c <= 97: fg = PALETTE[c - 90]
        elif 40 <= c <= 47: bg = PALETTE[c - 40]
        elif 100 <= c <= 107: bg = PALETTE[c - 100]
        elif c == 38: fg, i = color(codes, i)
        elif c == 48: bg, i = color(codes, i)
        i += 1
    return style()

text = sys.stdin.read()
sys.stdout.write(re.sub(r"\033\[([0-9;]*)m", lambda m: apply(m.group(1)), text))
