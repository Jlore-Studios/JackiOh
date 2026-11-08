#!/usr/bin/env python3
"""usage: splice.py <scratch.ts> -- puts each `// @@ <block>` section of the scratch file between the
`// <<block>>` and `// <</block>>` markers of sceneKit.ts, under a lock, after an esbuild syntax check."""
import fcntl, re, subprocess, sys
ROOT = "/home/runner/work/_temp/work/item-3392"
KIT = f"{ROOT}/apps/web/src/cards/art/sceneKit.ts"
scratch = open(sys.argv[1]).read()
sections = {}
for m in re.finditer(r"^// @@ (.+?)\n(.*?)(?=^// @@ |\Z)", scratch, re.S | re.M):
    sections[m.group(1).strip()] = m.group(2).rstrip("\n")
with open("/tmp/splice.lock", "w") as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    s = open(KIT).read()
    for name, body in sections.items():
        indent = "  " if name.endswith(" kit") else ""
        start = f"{indent}// <<{name}>>\n"
        end = f"{indent}// <</{name}>>"
        i = s.find(start)
        j = s.find(end)
        if i < 0 or j < 0:
            sys.exit(f"no markers for block {name!r}")
        s = s[: i + len(start)] + (body + "\n" if body else "") + s[j:]
    check = subprocess.run(
        ["node", "-e", "const e=require(process.argv[1]);let c='';process.stdin.on('data',d=>c+=d).on('end',()=>{try{e.transformSync(c,{loader:'ts'})}catch(x){console.error(x.message);process.exit(1)}})",
         f"{ROOT}/node_modules/.pnpm/esbuild@0.28.2/node_modules/esbuild"],
        input=s, text=True, capture_output=True)
    if check.returncode != 0:
        sys.exit("syntax error, sceneKit.ts left unchanged:\n" + check.stderr[-2000:])
    open(KIT, "w").write(s)
print("spliced:", ", ".join(sections))
