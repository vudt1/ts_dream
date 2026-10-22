#!/usr/bin/env python3
"""
Giải mã Data/Festival.Dat theo đúng thuật toán client (FUN_0051e00c + FUN_0051decc).

Nguồn: aLogin.exe (Delphi) — TFestival.Load @ 0x0051E00C, TFestival.Decrypt @ 0x0051DECC.
Xem client_special_event_handler/FESTIVAL_DAT_FORMAT.md để biết chi tiết.

Cách dùng:
    python3 tools/festival_dat_decrypt.py                     # giải mã & in tóm tắt
    python3 tools/festival_dat_decrypt.py --dump 07x            # hexdump bản ghi
"""
import os, sys, struct, datetime, re

PATH = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                    '..', 'Data', 'Festival.Dat')
PATH = os.path.normpath(PATH)
RS = 0x7EF          # 2031 byte / bản ghi
MAXREC = 0x16E     # 366 bản ghi tối đa (BoundErr)
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                   '..', 'festival_dat_decrypted')


def viscii_table():
    src = open(os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                            '..', 'src', 'encoding.rs'), encoding='utf-8').read()
    i = src.index('pub fn viscii_to_unicode'); j = src.index('pub fn unicode_to_viscii')
    V = {}
    for m in re.finditer(r"(0x[0-9A-Fa-f]{2})\s*=>\s*'(\\u\{[0-9A-Fa-f]+\}|\\?.)'", src[i:j]):
        b = int(m.group(1), 16); ch = m.group(2)
        if ch.startswith('\\u{'):
            ch = chr(int(ch[3:-1], 16))
        else:
            ch = {'\\n': '\n', '\\r': '\r', '\t': '\t', "\\'": "'", '\\\\': '\\'}.get(ch, ch)
        V[b] = ch
    return V


V = viscii_table()


def dec(bs):
    return ''.join(V.get(b, chr(b)) for b in bs)


def main():
    raw = open(PATH, 'rb').read()
    n = len(raw) // RS
    count = n - 3
    recs = [bytearray(raw[i * RS:(i + 1) * RS]) for i in range(n)]
    keyA = bytes(recs[n - 2])
    keyB = bytes(recs[n - 1])
    for i in range(1, count + 1):
        key = keyB if (i & 1) else keyA
        for j in range(RS):
            recs[i][j] ^= key[j]

    print(f'File        : {PATH}')
    print(f'FileSize    : {len(raw)} byte')
    print(f'RecordSize  : {RS} (0x7EF)')
    print(f'So ban ghi  : {n}')
    print(f'count (n-3) : {count}   -> TFestival+0xB5FA6 (word)')
    print(f'keyA = ban ghi [{n-2}] (dung cho ban ghi CHAN)')
    print(f'keyB = ban ghi [{n-1}] (dung cho ban ghi LE)')

    os.makedirs(OUT, exist_ok=True)
    for i in range(n):
        open(f'{OUT}/rec{i:02d}.bin', 'wb').write(bytes(recs[i]))

    print('\n=== NOI DUNG 7 BAN GHI DA GIAI MA ===')
    for i in range(1, count + 1):
        d = bytes(recs[i])
        dt = struct.unpack('<d', d[0:8])[0]
        w = struct.unpack('<H', d[8:10])[0]
        ln = d[0x0A]
        title = d[0x0B:0x0B + ln].decode('latin-1')
        t1 = d[0x1F:d.find(b'\x00', 0x1F)].decode('latin-1')
        e2 = d.find(b'\x00', 0x407)
        t2 = d[0x407:e2].decode('latin-1')
        try:
            ds = (datetime.datetime(1899, 12, 30) +
                  datetime.timedelta(days=dt)).strftime('%Y-%m-%d %H:%M:%S')
        except Exception:
            ds = '?'
        print(f'\n--- ban ghi {i} ---')
        print(f'  double @+0x00 = {dt!r}  (TDateTime -> {ds})')
        print(f'  word   @+0x08 = 0x{w:04X} ({w})')
        print(f'  tieu de @+0x0B ({ln} byte) = {dec(title.encode("latin-1"))}')
        print(f'  doan 1 @+0x1F ({len(t1)} byte) = {dec(t1.encode("latin-1"))[:180]}')
        print(f'  doan 2 @+0x407 ({len(t2)} byte) = {dec(t2.encode("latin-1"))[:180]}')

    if len(sys.argv) > 2 and sys.argv[1] == '--dump':
        k = int(sys.argv[2], 16)
        d = bytes(recs[k])
        for off in range(0, min(len(d), 0x60), 16):
            row = d[off:off + 16]
            print(f'{off:04X}  ' + ' '.join(f'{b:02X}' for b in row) + '   |' + dec(row) + '|')


if __name__ == '__main__':
    main()
