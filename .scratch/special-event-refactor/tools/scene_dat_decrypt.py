#!/usr/bin/env python3
"""
Giai ma Scene.Dat theo DUNG thuat toan client (FUN_0053542C, Round-6 Ghidra).

Xac nhan tu client:
  FUN_00535600 (loader):
      - doc file "Data\\Scene.Dat"
      - BlockRead vao self+0x18 : mang ban ghi 134 byte, TAT CA bat dau tu byte 0
        => file KHONG co header rieng; block 0 (134 byte dau) chinh la ban ghi 0
      - *(u16*)(self+0x04) = count
      - *(u16*)(self+0x18) = 0   <-- ep ban ghi 0 thanh sentinel "khong tim thay"
  FUN_0053542C (giai ma):
      for c = 1 .. count:
          w = (w ^ 0xEA6C) - 9   tai ban ghi +0x00          (map id)
          w = (w ^ 0xEA6C) - 9   tai ban ghi +0x18 .. +0x84 (55 word)
  FUN_00535864 (tra cuu):
      binary search so sanh (u16)ban ghi[+0x00] == mapId va (u8)ban ghi[+0x02] == kind
  FUN_0053558C: copy 134 byte ban ghi ra buffer
"""
import re, os, csv, json
from collections import Counter, defaultdict

SRC = '/mnt/d/VUDT/GIT_PCC/ts_dream/Data/Scene.Dat'
OUTDIR = '/mnt/d/VUDT/GIT_PCC/ts_dream/client_special_event_handler'
REC = 134
XOR = 0xEA6C
SUB = 9


def load_viscii():
    src = open('/mnt/d/VUDT/GIT_PCC/ts_dream/src/encoding.rs', encoding='utf-8').read()
    i0 = src.index('pub fn viscii_to_unicode')
    j0 = src.index('pub fn unicode_to_viscii', i0)
    V = {}
    pat = re.compile(r"(0x[0-9A-Fa-f]{2})\s*=>\s*('(?:\\u\{[0-9A-Fa-f]+\}|\\?.)')")
    for m in pat.finditer(src[i0:j0]):
        b = int(m.group(1), 16)
        lit = m.group(2)[1:-1]
        if lit.startswith('\\u{'):
            ch = chr(int(lit[3:-1], 16))
        else:
            ch = {'\\n': '\n', '\\r': '\r', '\\t': '\t', "\\'": "'", '\\\\': '\\'}.get(lit, lit)
        V[b] = ch
    # fallback cua client: b => b as char  (Latin-1 pass-through)
    for b in range(0x00, 0x100):
        V.setdefault(b, chr(b))
    return V


V = load_viscii()
MISSING = set()


def vdec(bs):
    out = []
    for b in bs:
        if b in V:
            out.append(V[b])
        else:
            MISSING.add(b)
            out.append('?')
    return ''.join(out)


def dec_word(w):
    return ((w ^ XOR) - SUB) & 0xFFFF


def decode(path=SRC):
    d = open(path, 'rb').read()
    n = len(d) // REC
    assert len(d) % REC == 0, (len(d), REC)
    recs = []
    for i in range(n):
        r = bytearray(d[i*REC:(i+1)*REC])
        raw_id = r[0] | (r[1] << 8)
        mid = 0 if i == 0 else dec_word(raw_id)
        r[0] = mid & 0xFF
        r[1] = (mid >> 8) & 0xFF
        for k in range(1, 0x38):          # word tai +0x18 .. +0x84
            off = 0x16 + k * 2
            w = r[off] | (r[off+1] << 8)
            v = dec_word(w)
            r[off] = v & 0xFF
            r[off+1] = (v >> 8) & 0xFF
        ln = r[3]
        name = vdec(bytes(r[4:4+ln])) if 0 < ln < 120 else ''
        kind = r[2]
        words = []
        for k in range(1, 0x38):
            off = 0x16 + k * 2
            words.append(r[off] | (r[off+1] << 8))
        recs.append(dict(idx=i, raw_id=raw_id, map_id=mid, kind=kind, name=name, words=words))
    return recs


def main():
    recs = decode()
    print(f'Soi ban ghi Scene.Dat: {len(recs)} (mien 134 byte, khong co header rieng)')
    print(f'byte VISCII chua biet: {sorted(MISSING)}')
    s = [r for r in recs if r["idx"] > 0]
    print(f'ban ghi 0 = sentinel (map id ep = 0)')
    print(f'map id nho nhat (ban ghi 1): {s[0]["map_id"]} "{s[0]["name"]}"')
    print(f'map id lon nhat (ban ghi cuoi): {s[-1]["map_id"]} "{s[-1]["name"]}"')
    ids = [r["map_id"] for r in s]
    print(f'map id trung lap: {len(ids)-len(set(ids))}')
    pairs = [(r["map_id"], r["kind"]) for r in s]
    print(f'(map id,kind) trung lap: {len(pairs)-len(set(pairs))}')
    bad = [(s[i]["idx"], s[i]["map_id"], s[i+1]["map_id"]) for i in range(len(s)-1)
           if s[i+1]["map_id"] < s[i]["map_id"]]
    print(f'cap khong tang danh: {len(bad)} -> {bad[:5]}')
    nz = [r for r in s if any(r["words"])]
    print(f'ban ghi co word giai ma khac 0: {len(nz)} / {len(s)}')
    if nz:
        for r in nz[:5]:
            print(f'   idx={r["idx"]} map={r["map_id"]} "{r["name"]}" words={r["words"][:8]}')
    print('kind pho bien:', Counter(r["kind"] for r in s).most_common(12))

    # ---- ghi catalog ----
    p = os.path.join(OUTDIR, 'data_map_catalog.tsv')
    with open(p, 'w', encoding='utf-8', newline='') as f:
        f.write('# Scene.Dat - catalog map (client doc tai Data/Scene.Dat)\n')
        f.write('# GIAI MA theo dung client (FUN_0053542C):\n')
        f.write('#   - ban ghi 134 byte, KHONG co header: file = 7481 x 134 byte\n')
        f.write('#   - ban ghi 0 = sentinel, client ep map id = 0\n')
        f.write('#   - map id = (u16 tai +0x00 ^ 0xEA6C) - 9\n')
        f.write('#   - kind   = u8  tai +0x02\n')
        f.write('#   - ten    = ShortString tai +0x03 (len) / +0x04 (text, VISCII 1.1)\n')
        f.write('#   - 55 word tai +0x18..+0x84 cung bi giai ma cung phep ((w^0xEA6C)-9)\n')
        f.write('# idx\tmap_id\tkind\tname\tw0\tw1\tw2\n')
        w = csv.writer(f, delimiter='\t', lineterminator='\n')
        for r in recs:
            w.writerow([r['idx'], r['map_id'], r['kind'], r['name'],
                        r['words'][0] if r['words'] else '',
                        r['words'][1] if len(r['words']) > 1 else '',
                        r['words'][2] if len(r['words']) > 2 else ''])
    print('da ghi', p)

    # ---- bang map id -> ten (gop nhieu kind) ----
    byid = defaultdict(list)
    for r in s:
        byid[r['map_id']].append(r)
    p2 = os.path.join(OUTDIR, 'data_mapid_to_name.tsv')
    with open(p2, 'w', encoding='utf-8', newline='') as f:
        f.write('# BANGS MAP ID -> TEN MAP (giai ma Scene.Dat)\n')
        f.write('# map_id\tso_ban_ghi\tkinds\tten(gop)\n')
        w = csv.writer(f, delimiter='\t', lineterminator='\n')
        for mid in sorted(byid):
            rs = byid[mid]
            kinds = ','.join(str(x['kind']) for x in sorted(rs, key=lambda z: z['kind']))
            names = []
            for x in rs:
                if x['name'] not in names:
                    names.append(x['name'])
            w.writerow([mid, len(rs), kinds, ' | '.join(names)])
    print('da ghi', p2, '-', len(byid), 'map id')

    # ---- noi voi eve.emg ----
    ev = os.path.join(OUTDIR, 'data_files/eve/eve_scene_index.tsv')
    if os.path.exists(ev):
        scenes = []
        for line in open(ev, encoding='utf-8'):
            if line.startswith('#') or line.startswith('scene_id'):
                continue
            c = line.rstrip('\n').split('\t')
            if c and c[0].isdigit():
                scenes.append(int(c[0]))
        hit = [x for x in scenes if x in byid]
        print(f'eve.emg: {len(scenes)} scene, {len(hit)} scene co map id khop Scene.Dat')
        p3 = os.path.join(OUTDIR, 'data_event_maps.tsv')
        with open(p3, 'w', encoding='utf-8', newline='') as f:
            f.write('# MAP SU KIEN (eve.emg scene_id) -> TEN MAP (Scene.Dat giai ma)\n')
            f.write('# scene_id(map_id)\tten_map\tkinds\tso_ban_ghi\tco_trong_eve\n')
            w = csv.writer(f, delimiter='\t', lineterminator='\n')
            for sid in sorted(set(scenes)):
                if sid in byid:
                    rs = byid[sid]
                    kinds = ','.join(str(x['kind']) for x in sorted(rs, key=lambda z: z['kind']))
                    names = []
                    for x in rs:
                        if x['name'] not in names:
                            names.append(x['name'])
                    w.writerow([sid, ' | '.join(names), kinds, len(rs), 'Y'])
                else:
                    w.writerow([sid, '', '', '', 'N'])
        print('da ghi', p3)


if __name__ == '__main__':
    main()
