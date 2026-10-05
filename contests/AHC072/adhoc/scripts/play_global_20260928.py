#!/usr/bin/env python3
"""Thin API recorder for manually chosen actions; contains no solver or search."""
import json
from pathlib import Path
import time
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/play_global_20260928"
URL = "http://127.0.0.1:5173/api/play/"
OUT.mkdir(exist_ok=True)


def api(path, body=None):
    req = urllib.request.Request(URL+path, data=None if body is None else json.dumps(body).encode(),
                                 headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req) as response:
        return json.load(response)


def sid(name):
    return json.loads((OUT / (name+".json")).read_text())["session_id"]


def create(name, *, case=None, input_text=None, output=None, label=""):
    body={"label": label or name, "actor": "ai"}
    if case is not None: body["case_name"]=case
    if input_text is not None: body["input"]=input_text
    if output is not None: body["output"]=output
    result=api("sessions",body)
    (OUT / (name+".json")).write_text(json.dumps(result,ensure_ascii=False,indent=2)+"\n")
    show(name)
    return result


def state(name,node=None):
    return api("sessions/"+sid(name)+"/state"+("?node_id="+node if node else ""))


def show(name,node=None):
    r=state(name,node)
    print(json.dumps({k:r[k] for k in ("session_id","node_id","revision","N","K","M","T","E","nests","towers")},ensure_ascii=False))
    return r


def mutate(name, method, fields):
    r=state(name)
    body={"expected_revision":r["revision"],"request_id":str(uuid.uuid4()),"actor":"ai",**fields}
    result=api("sessions/"+sid(name)+"/"+method,body)
    with (OUT / "actions.jsonl").open("a") as f:
        f.write(json.dumps({"at":time.time(),"name":name,"method":method,"request":body,"response":result},ensure_ascii=False)+"\n")
    return result


def step(name, moves, note=""):
    if isinstance(moves,str): moves=[line.split() for line in moves.strip().splitlines()]
    for index,(i,j,k,d,l) in enumerate(moves):
        action={"i":int(i),"j":int(j),"k":int(k),"d":d,"l":int(l)}
        result=mutate(name,"step",{"action":action,"note":note if index==0 else ""})
        print(json.dumps({k:result[k] for k in ("node_id","T","E","changed_cells","returned")},ensure_ascii=False))
    return result


def checkout(name,node):
    result=mutate(name,"checkout",{"node_id":node})
    print("checkout",name,node)
    return result


def note(name,text):
    return mutate(name,"note",{"text":text})


def compare(name,a,b):
    left,right=state(name,a),state(name,b)
    fields=("E","towers","received")
    equal=all(left[k]==right[k] for k in fields)
    result={"name":name,"a":a,"b":b,"same_board":equal,"T_a":left["T"],"T_b":right["T"],"E_a":left["E"],"E_b":right["E"]}
    print(json.dumps(result,ensure_ascii=False))
    with (OUT/"comparisons.jsonl").open("a") as f:f.write(json.dumps(result,ensure_ascii=False)+"\n")
    return result
