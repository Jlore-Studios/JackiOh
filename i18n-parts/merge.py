# Folds i18n-parts/*.json into apps/web/src/i18n/zh-CN.json: existing entries win, new ones append.
import json, sys, os, glob, re
repo = sys.argv[1]
target = os.path.join(repo, "apps/web/src/i18n/zh-CN.json")
d = json.load(open(target, encoding="utf8"))
TOKEN = re.compile(r"\{(\w+)\}")
added = 0
for part in sorted(glob.glob(os.path.join(repo, "i18n-parts", "*.json"))):
    frag = json.load(open(part, encoding="utf8"))
    for k, v in frag.items():
        vals = [v] if isinstance(v, str) else list(v.values())
        if any(sorted(TOKEN.findall(x)) != sorted(TOKEN.findall(k)) for x in vals):
            print(f"token mismatch in {os.path.basename(part)}: {k!r} -> {v!r}")
            continue
        if k in d:
            if d[k] != v:
                print(f"kept existing for {k!r}: {d[k]!r} (fragment {os.path.basename(part)} had {v!r})")
            continue
        d[k] = v
        added += 1
with open(target, "w", encoding="utf8") as f:
    json.dump(d, f, ensure_ascii=False, indent=2)
    f.write("\n")
print(f"added {added}, total {len(d)}")
