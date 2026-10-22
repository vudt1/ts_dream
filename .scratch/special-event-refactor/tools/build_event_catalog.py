#!/usr/bin/env python3
"""
Sinh catalog map (Scene.Dat) + NPC (Npc.dat) phuc vu module "su kien dac biet".

Usage:
    python3 tools/build_event_catalog.py            # tao data_map_catalog.tsv, data_event_maps.tsv, data_npc_catalog.tsv
"""
import os, re, struct, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REPO = os.path.normpath(os.path.join(ROOT, '..'))


def viscii_table():
    src = open(os.path.join(REPO, 'src', 'encoding.rs'), encoding='utf-8').read()
    i = src.index('pub fn viscii_to_unicode'); j = src.index('pub fn unicode_to_viscii')
    V = {}
    for m in re.finditer(r"(0x[0-9A-Fa-f]{2})\s*=>\s*'(\\u\{[0-9A-Fa-f]+\}|\\?.)'", src[i:j]):
        b = int(m.group(1), 16); ch = m.group(2)
        if ch.startswith('\\u{'):
            ch = chr(int(ch[3:-1], 16))
        else:
            ch = {'\\n': '\n', '\\r': '\r', '\\t': '\t', "\\'": "'", '\\\\': '\\'}.get(ch, ch)
        V[b] = ch
    for b in range(0x100):          # fallback cua client: b => b as char
        V.setdefault(b, chr(b))
    return V


V = viscii_table()


def dec(bs):
    return ''.join(V.get(b, chr(b)) for b in bs)


# ------------------------------------------------------------------ Scene.Dat
# Giai ma theo client (FUN_0053542C, Round-6 Ghidra):
#   - ban ghi 134 byte; file KHONG co header rieng (block 0 = sentinel)
#   - map id = (u16 tai +0x00 ^ 0xEA6C) - 9
#   - kind   = u8 tai +0x02
#   - ten    = ShortString tai +0x03 (len) / +0x04 (text VISCII)
#   - khoa tra cuu cua client = (mapId, kind)
RS = 134
XOR_KEY = 0xEA6C
XOR_SUB = 9


def dec_w(w):
    return ((w ^ XOR_KEY) - XOR_SUB) & 0xFFFF


def build_maps():
    d = open(os.path.join(REPO, 'Data', 'Scene.Dat'), 'rb').read()
    n = len(d) // RS
    rows = []
    for i in range(n):
        r = bytearray(d[i * RS:(i + 1) * RS])
        if i == 0:
            # ban ghi 0 = sentinel: client KHONG giai ma, ep map id = 0
            mid = 0
        else:
            mid = dec_w(r[0] | (r[1] << 8))
        r[0] = mid & 0xFF
        r[1] = (mid >> 8) & 0xFF
        for k in range(1, 0x38):
            off = 0x16 + k * 2
            v = dec_w(r[off] | (r[off + 1] << 8))
            r[off] = v & 0xFF
            r[off + 1] = (v >> 8) & 0xFF
        L = r[3]
        name = dec(bytes(r[4:4 + L])) if 0 < L < 120 else ''
        rows.append((i, name, mid, r[2]))
    with open(os.path.join(ROOT, 'data_map_catalog.tsv'), 'w', encoding='utf-8') as f:
        f.write('# Scene.Dat - catalog map (client doc tai Data/Scene.Dat)\n')
        f.write('# GIAI MA theo client (FUN_0053542C): ban ghi 134 byte, KHONG co header rieng;\n')
        f.write('#   map id = (u16 tai +0x00 ^ 0xEA6C) - 9 ; kind = u8 tai +0x02 ;\n')
        f.write('#   ten = ShortString tai +0x03 (len) / +0x04 (text VISCII 1.1)\n')
        f.write('# idx\tmap_id\tkind\tname\tw0\tw1\tw2\n')
        for i, name, mid, kind in rows:
            f.write(f'{i}\t{mid}\t{kind}\t{name}\t0\t0\t0\n')
    print('data_map_catalog.tsv:', len(rows), 'ban ghi')

    # bang map id -> ten
    byid = {}
    for i, name, mid, kind in rows:
        if i == 0:
            continue
        e = byid.setdefault(mid, {'n': 0, 'k': [], 'names': []})
        e['n'] += 1
        if kind not in e['k']:
            e['k'].append(kind)
        if name not in e['names']:
            e['names'].append(name)
    with open(os.path.join(ROOT, 'data_mapid_to_name.tsv'), 'w', encoding='utf-8') as f:
        f.write('# BANGS MAP ID -> TEN MAP (giai ma Scene.Dat)\n')
        f.write('# map_id\tso_ban_ghi\tkinds\tten(gop)\n')
        for mid in sorted(byid):
            e = byid[mid]
            f.write(f"{mid}\t{e['n']}\t{','.join(str(k) for k in sorted(e['k']))}\t{' | '.join(e['names'])}\n")
    print('data_mapid_to_name.tsv:', len(byid), 'map id')

    pat = re.compile(r'Hổ Lao|Lôi ?đài|LôiĐài|Cửu Thiên|Thủy ?chiến|đua thuyền|Thuyền rồng|'
                     r'Công ?thành|Thành Môn|Tháp Lôi|Niên ?Thú|Thiên Niên|Thất Tịch|Đoan Ngọ|'
                     r'Halloween|Quỷ ?Vương|Bạch Hổ Trảo|Hổ Trảo|Thôn Thực|Tỷ ?Võ|'
                     r'Ghi danh|Cầu Thất Tịch|Vương thành|Tiền tiêu|Điểm binh đài|'
                     r'Mê cung|Phật|Giới Kiều|Ngưu Chữ|Tiên Giới|Hội thi|Nguyên Tiêu|'
                     r'DoanViên|Chợ P-TS|Tiền tiêu', re.I)
    with open(os.path.join(ROOT, 'data_event_maps_by_name.tsv'), 'w', encoding='utf-8') as f:
        f.write('# Cac ban ghi Scene.Dat co TEN lien quan den SU KIEN DAC BIET\n')
        f.write('# (loc theo ten; danh sach day DU nam o data_mapid_to_name.tsv / data_event_maps.tsv)\n')
        f.write('# idx\tmap_id\tkind\tname\n')
        n_ev = 0
        for i, name, mid, kind in rows:
            if i and pat.search(name):
                f.write(f'{i}\t{mid}\t{kind}\t{name}\n')
                n_ev += 1
    print('data_event_maps_by_name.tsv:', n_ev, 'ban ghi')
    return rows


# ------------------------------------------------------------------ Npc.dat
NPC_RS = 92


def build_npcs():
    d = open(os.path.join(REPO, 'Data', 'Npc.dat'), 'rb').read()
    n = len(d) // NPC_RS
    rows = []
    for i in range(1, n):
        c = d[i * NPC_RS:i * NPC_RS + NPC_RS]
        L = c[0]
        if 0 < L <= 14:
            rows.append((i, dec(c[1 + 14 - L:1 + 14][::-1])))
    with open(os.path.join(ROOT, 'data_npc_catalog.tsv'), 'w', encoding='utf-8') as f:
        f.write('# Npc.dat - catalog ten NPC (client doc tai Data/Npc.dat, record 92 byte)\n')
        f.write('# npcId\tname\n')
        for i, name in rows:
            f.write(f'{i}\t{name}\n')
    print('data_npc_catalog.tsv:', len(rows), 'npc')

    pat = re.compile(r'Hổ|Lao|Lôi|Đài|Thủy|Chiến|Thuyền|Cửu|Thiên|Thành|Trảo|Niên|Quỷ|Tướng|'
                     r'Thủ ?Binh|Thủbinh|Bắc Tinh', re.I)
    ev = [r for r in rows if pat.search(r[1])]
    with open(os.path.join(ROOT, 'data_event_npcs.tsv'), 'w', encoding='utf-8') as f:
        f.write('# NPC co ten lien quan den SU KIEN DAC BIET\n')
        f.write('# npcId\tname\n')
        for i, name in ev:
            f.write(f'{i}\t{name}\n')
    print('data_event_npcs.tsv:', len(ev), 'npc')


if __name__ == '__main__':
    build_maps()
    build_npcs()
