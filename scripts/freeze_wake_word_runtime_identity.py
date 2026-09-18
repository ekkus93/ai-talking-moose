#!/usr/bin/env python3
"""Freeze sherpa-onnx V1 shared native runtime archive and library identities."""
from __future__ import annotations
import hashlib, json, struct, tarfile, tempfile, urllib.request
from pathlib import Path, PurePosixPath
VERSION="1.13.8"
BASE=f"https://github.com/k2-fsa/sherpa-onnx/releases/download/v{VERSION}"
PLATFORMS={
 "linux-x86_64":{"filename":f"sherpa-onnx-v{VERSION}-linux-x64-shared-lib.tar.bz2","architecture":"elf-x86_64"},
 "macos-arm64":{"filename":f"sherpa-onnx-v{VERSION}-osx-arm64-shared-lib.tar.bz2","architecture":"macho-arm64"},
}
def ident(data:bytes): return {"bytes":len(data),"sha256":hashlib.sha256(data).hexdigest()}
def arch(data:bytes):
 if len(data)>=20 and data[:4]==b"\x7fELF" and data[4:6]==b"\x02\x01" and struct.unpack_from("<H",data,18)[0]==62:return "elf-x86_64"
 if len(data)>=8 and data[:4]==b"\xcf\xfa\xed\xfe" and struct.unpack_from("<I",data,4)[0]==0x0100000C:return "macho-arm64"
 return None
def inspect(path:Path,expected:str):
 found=[]
 with tarfile.open(path,"r:bz2") as t:
  for m in t.getmembers():
   pure=PurePosixPath(m.name)
   if pure.is_absolute() or ".." in pure.parts: raise RuntimeError("unsafe archive member")
   if m.issym() or m.islnk(): continue
   if not m.isfile(): continue
   f=t.extractfile(m)
   if f is None: continue
   data=f.read(); a=arch(data)
   if a:
    if a!=expected: raise RuntimeError("wrong architecture in native member")
    found.append({"path":m.name,"architecture":a,**ident(data)})
 if not found: raise RuntimeError("no shared native libraries for expected architecture")
 return sorted(found,key=lambda x:str(x["path"]))
def main():
 result={"schema_version":1,"runtime_version":f"v{VERSION}","license":"Apache-2.0","platforms":{}}
 with tempfile.TemporaryDirectory(prefix="wake-runtime-freeze-") as td:
  root=Path(td)
  for platform,cfg in PLATFORMS.items():
   fn=cfg["filename"]; url=f"{BASE}/{fn}"; p=root/fn
   with urllib.request.urlopen(url,timeout=180) as response,p.open("wb") as out:
    while chunk:=response.read(1024*1024): out.write(chunk)
   result["platforms"][platform]={"source_url":url,"archive":{"filename":fn,**ident(p.read_bytes())},"install_root":f"runtime/sherpa-onnx/v{VERSION}/{platform}","files":inspect(p,cfg["architecture"])}
 print("WAKE_WORD_RUNTIME_IDENTITY="+json.dumps(result,sort_keys=True,separators=(",",":")))
 print(json.dumps(result,indent=2,sort_keys=True)); return 0
if __name__=="__main__": raise SystemExit(main())
