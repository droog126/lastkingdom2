import sys
sys.path.insert(0, r"F:\rustProject\lastkingdom2\tools")

import importlib.util
spec = importlib.util.spec_from_file_location("v5", r"F:\rustProject\lastkingdom2\tools\build_v5_cute.py")

import ast
src = open(r"F:\rustProject\lastkingdom2\tools\build_v5_cute.py", encoding='utf-8').read()
try:
    ast.parse(src)
    print("AST parse OK")
except SyntaxError as e:
    print(f"AST parse FAIL: line {e.lineno}: {e.msg}")
    print(f"  text: {e.text}")
