with open(r'F:\rustProject\lastkingdom2\tools\build_v5_cute.py','rb') as f:
    data = f.read()
target = b'uv_sphere("body"'
idx = data.find(target)
print('found at', idx)
if idx >= 0:
    print(repr(data[idx:idx+250]))
