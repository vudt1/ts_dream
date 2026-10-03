import json,sys,re,glob,os
V={}
src=open('/mnt/d/VUDT/GIT_PCC/ts_dream/src/encoding.rs',encoding='utf-8').read()
i=src.index('pub fn viscii_to_unicode'); j=src.index('pub fn unicode_to_viscii')
for m in re.finditer(r"(0x[0-9A-Fa-f]{2})\s*=>\s*'(\\u\{[0-9A-Fa-f]+\}|\\?.)'", src[i:j]):
    b=int(m.group(1),16); ch=m.group(2)
    if ch.startswith('\\u{'): ch=chr(int(ch[3:-1],16))
    else: ch={'\\n':'\n','\\r':'\r','\\t':'\t',"\\'":"'",'\\\\':'\\'}.get(ch,ch)
    V[b]=ch
for b in range(256):
    V.setdefault(b, chr(b) if (32<=b<127 or b>=0xA0) else f'\\x{b:02X}')
def dec(bs): return ''.join(V[b] for b in bs)
def load(p):
    d=[]
    for l in open(p):
        l=l.strip()
        if not l or l.startswith(';'): continue
        d+=[int(x,16) for x in l.split()]
    return d
IDX={}
# Hai corpus: ban goc (client_pseudo_c) va ban Round-4/5 (ts_decompile/redump).
# 238 file trung ten da doi chieu byte-for-byte -> giong nhau, nen gop la an toan.
# 75 file CHI co trong redump/ (lit_78A854, lit_796f*, lit_7980*, lit_799*, ...)
# nen phai index ca hai neu muon doc literal moi.
_CORPORA=[
    '/mnt/d/VUDT/GIT_PCC/ts_dream/client_pseudo_c',
    '/mnt/d/VUDT/GIT_PCC/test/ts_decompile/redump',
]
_paths=[]
for _d in _CORPORA:
    _paths += sorted(glob.glob(os.path.join(_d,'lit_*.hex')))
# corpus moi thang cho cu (Round-4 la ban moi hon)
_paths.sort(key=lambda p:(0 if 'ts_decompile' in p else 1, p))
for p in _paths:
    hdr=open(p).readline()
    m=re.search(r'([0-9a-fA-F]{6,8})\.\.([0-9a-fA-F]{6,8})',hdr)
    d=load(p)
    if m: start=int(m.group(1),16); end=int(m.group(2),16)
    else:
        mm=re.match(r'lit_([0-9a-fA-F]+)',os.path.basename(p)); start=int(mm.group(1),16); end=start+len(d)-1
    for i,b in enumerate(d):
        if start+i<=end: IDX[start+i]=b
def ansistrings(s,e):
    out=[];a=s
    while a+8<=e+1:
        if all(IDX.get(a+i)==255 for i in range(4)):
            ln=sum(IDX.get(a+4+k,0)<<(8*k) for k in range(4))
            if 0<ln<400 and a+8+ln<=e+1:
                bs=[IDX.get(a+8+k) for k in range(ln)]
                if all(x is not None for x in bs) and IDX.get(a+8+ln)==0:
                    out.append((a+8,ln,dec(bs))); a=a+8+ln+1; continue
        a+=1
    return out
def bare(s,e):
    out=[];a=s;cur=[]
    while a<=e:
        b=IDX.get(a)
        if b is None: break
        if b==0:
            if len(cur)>=3: out.append((a-len(cur),len(cur),dec(cur)))
            cur=[];a+=1;continue
        cur.append(b);a+=1
    return out
# Usage:
#   python3 tools_decode_literals.py 798ea4 799208
# Builds a VA->byte index from every client_pseudo_c/lit_*.hex (reading the
# address range out of each file's `; lit_XXXX_YYYY.hex A..B` header), decodes
# with the VISCII 1.1 table taken from src/encoding.rs::viscii_to_unicode
# (+ the same Latin-1 fallback the Rust function uses for 0xA0..0xFF), then
# prints Delphi AnsiString constants ([FF FF FF FF][len:u32LE][bytes][00]) and
# bare NUL-terminated strings found in the range.
if __name__=='__main__':
    s=int(sys.argv[1],16);e=int(sys.argv[2],16)
    print(f"### range 0x{s:08X}..0x{e:08X}")
    print("--- Delphi AnsiString consts (FF FF FF FF | len:4 | bytes | 00) ---")
    for a,l,t in ansistrings(s,e): print(f"0x{a:08X} len={l:3d}  {t}")
    print("--- bare NUL-terminated / shortstring region ---")
    for a,l,t in bare(s,e):
        if l>=3 and not t.startswith('Ữ'): print(f"0x{a:08X} len={l:3d}  {t}")
