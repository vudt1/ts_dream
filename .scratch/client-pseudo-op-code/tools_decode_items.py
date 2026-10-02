# Usage:  python3 tools_decode_items.py 'hôn|Lễ'
# Re-implements src/data/loaders/item.rs::ItemDatLoader (FIELD_LENGTH=370,
# dec8=(v^0x9A)-9, dec16=(v^0xEFC3)-9, name at chunk[1+20-len .. 1+20] reversed)
# and prints BOTH the VISCII decode and the cp1252 decode that the Rust loader
# currently uses, so the two can be diffed. Item names are VISCII, not CP1252:
# cp1252 mangles ~30% of the Vietnamese characters (see G23 in
# marriage_child_feature_flow.md).
import sys,struct,importlib.util
_spec=importlib.util.spec_from_file_location('dec', __file__.replace('tools_decode_items.py','tools_decode_literals.py'))
dec=importlib.util.module_from_spec(_spec); _spec.loader.exec_module(dec)
FL=370
data=open('/mnt/d/VUDT/GIT_PCC/ts_dream/Data/Item.dat','rb').read()
def d8(v): return ((v^0x9A)-9)&0xFF
def d16(v): return ((v^0xEFC3)-9)&0xFFFF
def d32(v): return ((v^0x0B80F4B4)-9)&0xFFFFFFFF
def viscii(bs): return ''.join(dec.V[b] for b in bs)
def cp1252(bs):
    try: return bs.decode('cp1252')
    except: return bs.decode('latin-1')
rows=[]
n=len(data)//FL
for i in range(1,n):
    c=data[i*FL:(i+1)*FL]
    nl=c[0]
    if 0<nl<=20:
        s=c[1+20-nl:1+20][::-1]
    else: s=b''
    kind=d8(c[21]); iid=d16(int.from_bytes(c[22:24],'little'))
    if iid==0: continue
    needlv=d8(c[196]) if len(c)>196 else 0
    rows.append((iid,viscii(s),cp1252(s),kind,nl))
print("records:",len(rows),file=sys.stderr)
import re
pat=re.compile(sys.argv[1]) if len(sys.argv)>1 else None
for iid,v,c,k,nl in rows:
    if pat is None or pat.search(v):
        print(f"{iid:6d}  VISCII={v!r:40s} CP1252={c!r:40s} kind={k}")
