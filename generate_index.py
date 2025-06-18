import ast
import sys
from pathlib import Path

repo_root = Path('.')

# Determine internal modules from repo structure
internal_modules = set()
for path in repo_root.rglob('*.py'):
    parts = path.relative_to(repo_root).parts
    if parts:
        # add the top-level package name
        internal_modules.add(parts[0])
        # add intermediate package names and module name itself
        for part in parts[:-1]:
            internal_modules.add(part)
        internal_modules.add(path.stem)

# built-in modules
try:
    builtin_mods = set(sys.stdlib_module_names)
except AttributeError:
    import pkgutil, sys as _sys
    builtin_mods = {mod.name for mod in pkgutil.iter_modules() if mod.module_finder.path in _sys.path}

def is_third_party(mod: str) -> bool:
    """Return True if module name appears to be third-party."""
    if not mod:
        return False
    if mod.startswith('.'):
        return False
    base = mod.split('.')[0]
    return base not in builtin_mods and base not in internal_modules

index = []
third_party_imports = set()

class IndexVisitor(ast.NodeVisitor):
    def __init__(self):
        self.current = []
        self.functions = []
        self.classes = {}
        self.class_stack = []

    def visit_FunctionDef(self, node: ast.FunctionDef):
        name = '.'.join(self.current + [node.name])
        if not self.class_stack:
            self.functions.append(
                (name, node.lineno, getattr(node, 'end_lineno', node.lineno))
            )
        self.current.append(node.name)
        self.generic_visit(node)
        self.current.pop()

    def visit_ClassDef(self, node: ast.ClassDef):
        name = '.'.join(self.current + [node.name])
        methods = {}
        for n in node.body:
            if isinstance(n, ast.FunctionDef):
                methods[n.name] = (n.lineno, getattr(n, 'end_lineno', n.lineno))
        self.classes[name] = (
            node.lineno,
            getattr(node, 'end_lineno', node.lineno),
            methods,
        )
        self.class_stack.append(node.name)
        self.current.append(node.name)
        self.generic_visit(node)
        self.current.pop()
        self.class_stack.pop()

    def visit_If(self, node: ast.If):
        # detect if __name__ == '__main__'
        if not self.current:
            cmp = node.test
            if (
                isinstance(cmp, ast.Compare)
                and len(cmp.ops) == 1
                and isinstance(cmp.ops[0], ast.Eq)
                and isinstance(cmp.left, ast.Name)
                and cmp.left.id == '__name__'
                and len(cmp.comparators) == 1
                and isinstance(cmp.comparators[0], ast.Constant)
                and cmp.comparators[0].value == '__main__'
            ):
                self.functions.append(
                    ("__main__", node.lineno, getattr(node, 'end_lineno', node.lineno))
                )
        self.generic_visit(node)


for py_file in sorted(repo_root.rglob('*.py')):
    rel_path = py_file.relative_to(repo_root)
    with py_file.open('r', encoding='utf-8') as f:
        try:
            tree = ast.parse(f.read())
        except Exception as e:
            print(f'Failed parsing {rel_path}: {e}', file=sys.stderr)
            continue

    module_info = {
        'file': str(rel_path),
        'classes': {},
        'functions': [],
        'constants': []
    }

    # constants only from module-level assignments
    for node in tree.body:
        if isinstance(node, (ast.Assign, ast.AnnAssign)):
            targets = []
            if isinstance(node, ast.Assign):
                targets = [t.id for t in node.targets if isinstance(t, ast.Name)]
            else:
                if isinstance(node.target, ast.Name):
                    targets = [node.target.id]
            for name in targets:
                if name.isupper():
                    module_info['constants'].append(name)

    visitor = IndexVisitor()
    visitor.visit(tree)
    module_info['functions'] = visitor.functions
    module_info['classes'] = visitor.classes

    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                if is_third_party(alias.name):
                    third_party_imports.add(alias.name.split('.')[0])
        elif isinstance(node, ast.ImportFrom):
            if node.level and node.level > 0:
                continue
            if node.module and is_third_party(node.module):
                third_party_imports.add(node.module.split('.')[0])

    index.append(module_info)

# Write INDEX.md
with open('INDEX.md', 'w', encoding='utf-8') as f:
    f.write('# Project Python Index\n\n')
    f.write('## Third-party Libraries\n\n')
    for lib in sorted(third_party_imports):
        f.write(f'- {lib}\n')

    f.write('\n## Modules\n')
    for mod in index:
        f.write(f"\n### {mod['file']}\n")
        if mod['constants']:
            f.write('* Constants: ' + ', '.join(mod['constants']) + '\n')
        if mod['functions']:
            func_items = [
                f"{name} (L{start}-{end})" for name, start, end in mod['functions']
            ]
            f.write('* Functions: ' + ', '.join(func_items) + '\n')
        if mod['classes']:
            f.write('* Classes:\n')
            for cls, (cls_start, cls_end, methods) in mod['classes'].items():
                if methods:
                    method_list = ', '.join(
                        f"{m} (L{start}-{end})" for m, (start, end) in methods.items()
                    )
                    f.write(f'  - {cls} (L{cls_start}-{cls_end}): {method_list}\n')
                else:
                    f.write(f'  - {cls} (L{cls_start}-{cls_end})\n')
