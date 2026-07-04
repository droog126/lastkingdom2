from pathlib import Path


with (Path(__file__).resolve().parent / "build_v5_cute.py").open("rb") as f:
    data = f.read()

print("Size:", len(data))
print("First 4 bytes hex:", data[:4].hex())

if data[:3] == b'\xef\xbb\xbf':
    print("UTF-8 BOM")

try:
    data.decode('utf-8', errors='strict')
    print("Strict UTF-8 OK")
except UnicodeDecodeError as e:
    print(f"UTF-8 fail at byte {e.start}: {data[e.start:e.start+10].hex()}")

try:
    data.decode('cp1252', errors='strict')
    print("CP1252 OK")
except UnicodeDecodeError as e:
    print(f"CP1252 fail at byte {e.start}")
