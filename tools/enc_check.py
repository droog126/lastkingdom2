with open(r"F:\rustProject\lastkingdom2\tools\build_v5_cute.py", 'rb') as f:
    data = f.read()
# Check encoding
print("Size:", len(data))
print("First 4 bytes hex:", data[:4].hex())
# utf-8 BOM?
if data[:3] == b'\xef\xbb\xbf':
    print("UTF-8 BOM")
# try utf-8 strict
try:
    data.decode('utf-8', errors='strict')
    print("Strict UTF-8 OK")
except UnicodeDecodeError as e:
    print(f"UTF-8 fail at byte {e.start}: {data[e.start:e.start+10].hex()}")
# try cp1252/latin-1
try:
    data.decode('cp1252', errors='strict')
    print("CP1252 OK")
except UnicodeDecodeError as e:
    print(f"CP1252 fail at byte {e.start}")
