from pathlib import Path


with (Path(__file__).resolve().parent / "build_v5_cute.py").open("rb") as f:
    data = f.read()
target = b'uv_sphere("body"'
idx = data.find(target)
print('found at', idx)
if idx >= 0:
    print(repr(data[idx:idx+250]))
