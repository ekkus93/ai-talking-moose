#!/usr/bin/env python3
"""Freeze sherpa-onnx V1 shared C API runtime archive and library identities."""
from __future__ import annotations
import hashlib, json, struct, tarfile, tempfile, urllib.request
from pathlib import Path, PurePosixPath
VERSION = "1.13.8"
BASE = f"https://github.com/k2-fsa/sherpa-onnx/releases/download/v{VERSION}"
PLATFORMS = {
 "linux-x86_64":{"filename":f"sherpa-onnx-v{VERSION}-linux-x64-shared.tar.bz2","architecture":"elf-x86_64","required":("libonnxruntime.so","libsherpa-onnx-c-api.so")},
 "macos-arm64":{"filename":f"sherpa-onnx-v{VERSION}-osx-arm64-shared.tar.bz2","architecture":"macho-arm64","required":("libonnxruntime.dylib","libsherpa-onnx-c-api.dylib")},
}
def identity_bytes(data: bytes): return {"bytes":len(data),"sha256":hashlib.sha256(data).hexdigest()}
def architecture(data: bytes):
 if len(data)>=20 and data[:4]==b"\x7fELF" and data[4:6]==b"\x02\x01" and struct.unpack_from("<H",data,18)[0]==62: return "elf-x86_64"
 if len(data)>=8 and data[:4]==b"\xcf\xfa\xed\xfe" and struct.unpack_from("<I",data,4)[0]==0x0100000C: return "macho-arm64"
 return None
def inspect_archive(path: Path, expected: str, required: tuple[str, ...]):
 found=[]
 with tarfile.open(path,mode="r:bz2") as archive:
  for member in archive.getmembers():
   pure=PurePosixPath(member.name)
   if pure.is_absolute() or ".." in pure.parts or member.issym() or member.islnk(): raise RuntimeError("unsafe archive member")
   if not member.isfile() or pure.name not in required: continue
   source=archive.extractfile(member)
   if source is None: raise RuntimeError("missing native member")
   data=source.read(); arch=architecture(data)
   if arch!=expected: raise RuntimeError("wrong architecture in native member")
   found.append({"path":member.name,"architecture":arch,**identity_bytes(data)})
 if {PurePosixPath(item["path"]).name for item in found}!=set(required): raise RuntimeError("missing required shared runtime library")
 return sorted(found,key=lambda x:str(x["path"]))
def main():
 result={"schema_version":1,"runtime_version":f"v{VERSION}","license":"Apache-2.0","platforms":{}}
 with tempfile.TemporaryDirectory(prefix="wake-runtime-freeze-") as td:
  root=Path(td)
  for platform,cfg in PLATFORMS.items():
   filename=cfg["filename"]; url=f"{BASE}/{filename}"; path=root/filename
   with urllib.request.urlopen(url,timeout=120) as response,path.open("wb") as out:
    while chunk:=response.read(1024*1024): out.write(chunk)
   result["platforms"][platform]={"source_url":url,"archive":{"filename":filename,**identity_bytes(path.read_bytes())},"install_root":f"runtime/sherpa-onnx/v{VERSION}/{platform}","files":inspect_archive(path,cfg["architecture"],cfg["required"])}
 print("WAKE_WORD_RUNTIME_IDENTITY="+json.dumps(result,sort_keys=True,separators=(",",":")))
 print(json.dumps(result,indent=2,sort_keys=True))
 return 0
if __name__=="__main__": raise SystemExit(main())
