"""Pinned external harness. No network operations; fetch/install are explicit."""
import argparse
import collections
import json
import pathlib
import subprocess
import sys
import warnings
import tempfile
import xml.etree.ElementTree as ET

ROOT = pathlib.Path(__file__).resolve().parents[1]
CACHE = ROOT / "tests/external/cache"
REPORTS = ROOT / "tests/external/reports"

# Codes are grouped by analysis phase explicitly; their prefix denotes default severity.
PARSE_FAILURE_CODES = frozenset(["E001","E002","E003","E004","E005","E006","E007","E008","E031","E032","E033","E034"])
NORMALIZATION_FAILURE_CODES = frozenset(["E009","E010","E011","E012","E013","E014","E015","E016","E017"])

def verify(name):
    lock = json.loads((ROOT / "tests/external/repos.json").read_text())
    repo = next(r for r in lock if r["name"] == name)
    path = CACHE / name
    result = subprocess.run(["git", "-c", f"safe.directory={path.as_posix()}", "-C", str(path), "rev-parse", "HEAD"], capture_output=True, text=True, check=True)
    if result.stdout.strip() != repo["commit"]:
        raise RuntimeError(f"{name}: revision differs from pinned commit")
    return path

def value(resolved):
    return resolved["value"] if resolved else None

def token(enum):
    return enum.value if enum is not None else None

def projection_status(left, right):
    if left == right:
        return "PASS"
    adjusted=json.loads(json.dumps(left))
    for r in adjusted:
        if r["access"] is None:
            r["access"]="read-write"
        for f in r["fields"]:
            if f["access"] is None:
                f["access"]="read-write"
    return "EXPECTED DIFFERENCE" if adjusted==json.loads(json.dumps(right)) else "SVD-LINT BUG"

def enum_values(group, width, nodes):
    origin = group["values"]["origin"]["declaration"]
    source_items = list(nodes[origin].findall("enumeratedValue")) if origin is not None else []
    result, defaults = [], []
    for i, item in enumerate(group["values"]["value"]):
        if item.get("is_default") is True:
            defaults.append(item["name"])
            continue
        number = item.get("value")
        raw = source_items[i].findtext("value", "") if i < len(source_items) else ""
        raw = raw.strip().lower()
        bits = raw[1:] if raw.startswith("#") else raw[2:] if raw.startswith("0b") else ""
        if "x" in bits:
            if width > 12:
                raise ValueError("wildcard projection budget: width > 12")
            candidates = [0]
            for bit in bits:
                candidates = [(n << 1) | b for n in candidates for b in ([0, 1] if bit == "x" else [int(bit)])]
            result.extend((item["name"], n) for n in candidates)
        elif number is not None:
            result.append((item["name"], number))
    if defaults:
        if width > 12:
            raise ValueError("default projection budget: width > 12")
        used = {n for _, n in result}
        result.extend((defaults[0], n) for n in range(1 << width) if n not in used)
    return sorted(set(result))

def project_core(ir, path):
    nodes = list(ET.parse(path).getroot().iter())
    rows = []
    def generated_name(name, origin):
        for dimension in origin["arrays"]:
            if "[%s]" in dimension["template"]:
                name=name.replace("[" + dimension["index"] + "]", dimension["index"])
        return name
    for p in ir["peripherals"]:
        for r in p["registers"]:
            name = generated_name(r["name"].removeprefix(p["name"] + "."),r["origin"])
            fields = []
            for f in r["fields"]:
                groups = [{"usage":value(g["usage"]) or "read-write", "values":enum_values(g, f["bit_width"], nodes)} for g in f["enumerated_values"]]
                fields.append({"name":generated_name(f["name"],f["origin"]), "offset":f["bit_offset"], "width":f["bit_width"], "access":value(f["access"]), "enums":sorted(groups, key=lambda g:g["usage"])})
            rows.append({"peripheral":generated_name(p["name"],p["origin"]), "path":name, "address":r["address"], "size":value(r["size"]), "access":value(r["access"]), "fields":sorted(fields,key=lambda f:f["name"])})
    return sorted(rows,key=lambda r:(r["peripheral"],r["path"]))

def project_reference(device):
    from svdsuite.model import Cluster
    rows = []
    def walk(p, items, scope=""):
        for item in items:
            path = scope + item.name
            if isinstance(item, Cluster):
                walk(p, item.registers_clusters, path + ".")
                continue
            fields=[]
            for f in item.fields:
                groups=[]
                for g in f.enumerated_value_containers:
                    values=sorted(set((e.parsed.name, e.value) for e in g.enumerated_values))
                    groups.append({"usage":token(g.usage), "values":values})
                fields.append({"name":f.name, "offset":f.bit_offset, "width":f.bit_width, "access":token(f.access), "enums":sorted(groups,key=lambda g:g["usage"])})
            rows.append({"peripheral":p.name, "path":path, "address":item.base_address, "size":item.size, "access":token(item.access), "fields":sorted(fields,key=lambda f:f["name"])})
    for p in device.peripherals:
        walk(p, p.registers_clusters)
    return sorted(rows,key=lambda r:(r["peripheral"],r["path"]))

def compare(args):
    repo=verify("SVDSuite")
    sys.path.insert(0, str(repo / "src"))
    from svdsuite import Process
    paths=sorted((repo / "tests/svd").rglob("*.svd"))
    if args.limit:
        paths=paths[:args.limit]
    rows=[]
    known=json.loads((ROOT / "tests/external/differences.json").read_text())
    for path in paths:
        relative=path.relative_to(repo).as_posix()
        output=subprocess.run([args.binary, "dump-ir", str(path), "--format", "json"],capture_output=True,text=True,timeout=60)
        if output.returncode not in [0, 1]:
            raise RuntimeError(f"svd-lint crashed or could not analyze {relative}: {output.returncode}")
        diagnostic_codes=[]
        try:
            ir=json.loads(output.stdout)
            if not isinstance(ir,dict) or "peripherals" not in ir:
                diagnostic_codes=[d["code"] for d in ir.get("diagnostics",[])]
                ir=None
        except json.JSONDecodeError:
            ir=None
        reference_error=None
        with warnings.catch_warnings(record=True) as caught:
            try:
                reference=Process.from_svd_file(str(path)).get_processed_device()
            except Exception as error:
                reference=None
                reference_error=repr(error)
        row={"file":relative,"reference_warnings":[str(w.message) for w in caught]}
        if ir is None and reference is None:
            row.update(status="PASS",reason="both implementations reject input")
        elif reference is None:
            row.update(status="REFERENCE AMBIGUITY",reason=reference_error)
        elif ir is None:
            if "E016" in diagnostic_codes:
                row.update(status="EXPECTED DIFFERENCE",reason="svd-lint requires an effective register size; reference supplies or infers a default",diagnostics=diagnostic_codes)
                # Also compare a labelled variant with an explicit common size.
                # The original result is kept; neither original parser input nor
                # the library's inheritance policy is changed.
                tree=ET.parse(path)
                document=tree.getroot()
                if document.find("size") is None:
                    size=ET.Element("size")
                    size.text=document.findtext("width","32")
                    document.insert(list(document).index(document.find("peripherals")),size)
                    with tempfile.TemporaryDirectory() as temporary:
                        adjusted_path=pathlib.Path(temporary)/"explicit-size.svd"
                        tree.write(adjusted_path,encoding="utf-8")
                        variant=subprocess.run([args.binary,"dump-ir",str(adjusted_path),"--format","json"],capture_output=True,text=True,timeout=60)
                        variant_ir=json.loads(variant.stdout)
                        with warnings.catch_warnings():
                            warnings.simplefilter("ignore")
                            adjusted_ref=Process.from_svd_file(str(adjusted_path)).get_processed_device()
                        if "peripherals" in variant_ir:
                            left=project_core(variant_ir,adjusted_path)
                            right=project_reference(adjusted_ref)
                            row["explicit_size_variant"]={"status":projection_status(left,right),"core":left,"reference":right}
            else:
                row.update(status="SVD-LINT BUG",reason="reference resolves a model, svd-lint has no IR", diagnostics=diagnostic_codes)
        else:
            try:
                left=project_core(ir,path)
                right=project_reference(reference)
                if left==right:
                    row.update(status="PASS")
                else:
                    # Absent access is deliberately preserved, unlike reference defaults.
                    adjusted=json.loads(json.dumps(left))
                    for r in adjusted:
                        if r["access"] is None:
                            r["access"]="read-write"
                        for f in r["fields"]:
                            if f["access"] is None:
                                f["access"]="read-write"
                    if adjusted==json.loads(json.dumps(right)):
                        row.update(status="EXPECTED DIFFERENCE",reason="svd-lint preserves absent access; reference inserts read-write")
                    else:
                        row.update(status="SVD-LINT BUG",reason="semantic projections differ",core=left,reference=right)
            except ValueError as error:
                row.update(status="REFERENCE AMBIGUITY",reason=str(error))
        if relative in known and row["status"]!="PASS":
            row.update(status=known[relative]["status"],reason=known[relative]["reason"])
        if row.get("explicit_size_variant",{}).get("status")=="SVD-LINT BUG":
            if relative in known:
                row["explicit_size_variant"].update(status=known[relative]["status"],reason=known[relative]["reason"])
            else:
                row.update(status="SVD-LINT BUG",reason="unreviewed difference in explicit-size variant")
        rows.append(row)
    return {"version":1,"reference_commit":"fb45cbd1f3cef157a715f4de28ecc2e548cae47e","summary":dict(sorted(collections.Counter(r["status"] for r in rows).items())),"variant_summary":dict(sorted(collections.Counter(r["explicit_size_variant"]["status"] for r in rows if "explicit_size_variant" in r).items())),"cases":rows}

def data(args):
    repo=verify("cmsis-svd-data")
    source_root=repo / "data"
    paths=sorted(source_root.rglob("*.svd"))
    # Round-robin vendors rather than only the alphabetically first vendor.
    vendors=collections.defaultdict(collections.deque)
    for path in paths:
        vendors[path.relative_to(source_root).parts[0]].append(path)
    selected=[]
    while any(vendors.values()) and (not args.limit or len(selected)<args.limit):
        for vendor in sorted(vendors):
            if vendors[vendor] and (not args.limit or len(selected)<args.limit):
                selected.append(vendors[vendor].popleft())
    totals=collections.Counter()
    by_code=collections.Counter()
    by_vendor={}
    for path in sorted(selected):
        result=subprocess.run([args.binary,"check",str(path),"--format","json","--profile","strict"],capture_output=True,text=True,timeout=120)
        if result.returncode not in [0,1,2]:
            raise RuntimeError(f"svd-lint crashed on {path}: {result.returncode}")
        report=json.loads(result.stdout)
        diagnostics=report["diagnostics"]
        vendor=path.relative_to(source_root).parts[0]
        local=by_vendor.setdefault(vendor,collections.Counter())
        for target in [totals,local]:
            target["files"]+=1
            target["successfully_parsed"]+=not any(d["code"] in PARSE_FAILURE_CODES for d in diagnostics)
            target["normalization_failures"]+=any(d["code"] in NORMALIZATION_FAILURE_CODES for d in diagnostics)
            target["errors"]+=sum(d["severity"]=="error" for d in diagnostics)
            target["warnings"]+=sum(d["severity"]=="warning" for d in diagnostics)
        by_code.update(d["code"] for d in diagnostics)
    return {"version":1,"sample_limit":args.limit,"summary":dict(totals),"diagnostics_by_code":dict(sorted(by_code.items())),"diagnostics_by_vendor":{v:dict(n) for v,n in sorted(by_vendor.items())},"interpretation":"Warnings are observations, not automatically SVD defects."}

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--binary",default=str(ROOT / "target/debug" / ("svd-lint.exe" if sys.platform=="win32" else "svd-lint")))
    parser.add_argument("--suite",choices=["SVDSuite","data","all"],default="all")
    parser.add_argument("--limit",type=int,default=100)
    parser.add_argument("--allow-unreviewed",action="store_true",help="Write reports without gating unreviewed differences.")
    args=parser.parse_args()
    REPORTS.mkdir(parents=True,exist_ok=True)
    bug=False
    if args.suite in ["SVDSuite","all"]:
        report=compare(args)
        (REPORTS / "svdsuite.json").write_text(json.dumps(report,indent=2,ensure_ascii=False)+"\n",encoding="utf8")
        print(json.dumps(report["summary"],sort_keys=True))
        bug=report["summary"].get("SVD-LINT BUG",0)>0
    if args.suite in ["data","all"]:
        report=data(args)
        (REPORTS / "cmsis-svd-data.json").write_text(json.dumps(report,indent=2,ensure_ascii=False)+"\n",encoding="utf8")
        print(json.dumps(report["summary"],sort_keys=True))
    return 1 if bug and not args.allow_unreviewed else 0

if __name__=="__main__":
    sys.exit(main())

