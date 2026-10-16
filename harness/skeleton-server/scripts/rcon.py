#!/usr/bin/env python3
"""Minimal RCON client (stdlib only). usage: rcon.py 'cmd' ['cmd2' ...]"""
import socket, struct, sys
def pkt(i, t, body): 
    b = struct.pack('<ii', i, t) + body.encode() + b'\0\0'
    return struct.pack('<i', len(b)) + b
def rd(s):
    h = b''
    while len(h) < 4: h += s.recv(4 - len(h))
    n = struct.unpack('<i', h)[0]; d = b''
    while len(d) < n: d += s.recv(n - len(d))
    return struct.unpack('<ii', d[:8]) + (d[8:-2].decode(errors='replace'),)
def run(cmds, port=25577, pw='skeleton'):
    s = socket.create_connection(('127.0.0.1', port)); s.settimeout(120)
    s.sendall(pkt(1, 3, pw)); rd(s)
    out = []
    for c in cmds:
        s.sendall(pkt(2, 2, c)); out.append(rd(s)[2])
    s.close(); return out
if __name__ == '__main__':
    for o in run(sys.argv[1:]): print(o)
