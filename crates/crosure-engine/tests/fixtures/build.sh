#!/bin/sh
# Rebuilds the fixtures. Output is committed so tests need no compiler.
set -e
cd "$(dirname "$0")"
cc -O0 -fno-stack-protector -o crackme-x64 crackme.c
cp crackme-x64 crackme-x64-stripped
strip crackme-x64-stripped
x86_64-w64-mingw32-gcc -O0 -fno-stack-protector -s -o crackme-x64.exe crackme.c
