import sys
from pathlib import Path


HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import importlib.util
target = HERE / "build_v5_cute.py"
spec = importlib.util.spec_from_file_location("v5", target)

import ast
src = target.read_text(encoding="utf-8")
try:
    ast.parse(src)
    print("AST parse OK")
except SyntaxError as e:
    print(f"AST parse FAIL: line {e.lineno}: {e.msg}")
    print(f"  text: {e.text}")
