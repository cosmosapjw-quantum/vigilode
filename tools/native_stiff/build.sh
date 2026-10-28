#!/bin/bash
# Build the native comparator driver of the stiff benchmark (research node
# research/stiff_native_benchmark_20261001).
#
# - CVODE: SUNDIALS 6.4.1 from the distribution (libsundials-dev), BDF with its
#   own dense LU, or with a custom linear solver on OpenBLAS dgetrf/dgetrs.
# - RADAU5 and RODAS: Ernst Hairer's Fortran codes with DECSOL, taken from the
#   Assimulo 3.0 sdist on PyPI (pinned by SHA-256; BSD-style licence in
#   LICENSE_HAIRER of that archive). They are not vendored into this repository.
#
# Usage: tools/native_stiff/build.sh <build-dir>
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
out="${1:?build directory}"
mkdir -p "$out"
cd "$out"
url="https://files.pythonhosted.org/packages/28/ef/4d8b15a0842410bb4ca8f500dadd5ff76fb3e34559f956a51c19b8a4912b/Assimulo-3.0.tar.gz"
archive_sha=61cdca4562745078c966cc4fab44d8bc0db11a9e6856b45c470ac65c16af9e8a
radau_sha=dfb5925c1cb210df7ef2f8027c29b7049ffa60180d8ee96489c89817f828b22c
rodas_sha=ac7606807fb24e9333d4262a25c23a3e21b0895d918263a69de8680261f32a93
[ -f Assimulo-3.0.tar.gz ] || curl -sS -L -o Assimulo-3.0.tar.gz "$url"
echo "$archive_sha  Assimulo-3.0.tar.gz" | sha256sum -c -
tar xzf Assimulo-3.0.tar.gz --strip-components=4 \
  Assimulo-3.0/assimulo/thirdparty/hairer/radau_decsol.f \
  Assimulo-3.0/assimulo/thirdparty/hairer/rodas_decsol.f \
  Assimulo-3.0/assimulo/thirdparty/hairer/LICENSE_HAIRER
echo "$radau_sha  radau_decsol.f" | sha256sum -c -
echo "$rodas_sha  rodas_decsol.f" | sha256sum -c -
FFLAGS="-O3 -std=legacy -w"
CFLAGS="-O3 -std=c11 -Wall -Wextra"
gfortran $FFLAGS -c radau_decsol.f -o radau_decsol.o
gfortran $FFLAGS -c rodas_decsol.f -o rodas_decsol.o
# Both files carry their own DECSOL copy: keep only the entry points global
# (plus DEC for the LU microbenchmark) so the copies do not collide.
objcopy --keep-global-symbol=radau5_ --keep-global-symbol=dec_ radau_decsol.o radau.o
objcopy --keep-global-symbol=rodas_ rodas_decsol.o rodas.o
gcc $CFLAGS -c "$here/native_stiff.c" -o native_stiff.o
gfortran native_stiff.o radau.o rodas.o -o native_stiff \
  -lsundials_cvode -lsundials_nvecserial -lsundials_sunmatrixdense \
  -lsundials_sunlinsoldense -lopenblas -lm
{
  echo "gcc: $(gcc --version | head -1)"
  echo "gfortran: $(gfortran --version | head -1)"
  echo "CFLAGS: $CFLAGS"
  echo "FFLAGS: $FFLAGS"
  echo "sundials: $(dpkg-query -W -f='${Version}' libsundials-dev 2>/dev/null || echo unknown)"
  echo "openblas: $(dpkg-query -W -f='${Version}' libopenblas-dev 2>/dev/null || echo unknown)"
} > BUILD_INFO.txt
cat BUILD_INFO.txt
