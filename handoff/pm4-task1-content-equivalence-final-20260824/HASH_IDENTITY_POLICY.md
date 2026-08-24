# Hash and artifact identity policy

## Governing rule

> A SHA mismatch is a byte-identity signal, not by itself a scientific-integrity
> failure. Classify the object before assigning severity.

The following identities are distinct:

```text
Git object identity
!= working-tree byte identity
!= regenerated artifact identity
!= numerical equivalence
!= packaging identity
```

## 1. Git source authority — hard gate

For tracked source, use Git first:

```bash
git rev-parse HEAD
git rev-parse HEAD^{tree}
git ls-tree HEAD -- path/to/file
git diff -- path/to/file
git check-attr -a -- path/to/file
git ls-files --eol -- path/to/file
```

If the same commit and tree are present, Git blob content is the source
authority. Do not separately reinterpret a GitHub-to-local fetch as an
untrusted archive transfer.

If working-tree bytes differ from the indexed/committed content, investigate
EOL conversion, clean/smudge filters, LFS hydration, generators, or formatter
rewrites. This is diagnostic until an unexplained source change would enter the
build or commit.

## 2. Immutable external input/data — hard gate

External datasets, frozen calibration tables, immutable fixtures, or paper
inputs retain cryptographic hashes and provenance. A stale provenance pointer is
not excused by matching downstream output.

## 3. Deterministic generated evidence — conditional hard gate

An exact hash is hard only when byte-identical reproducibility is an explicit
claim. The generation path must then canonicalize order, timestamps, uid/gid,
permissions, locale, compression metadata, build path, and serializer details.

## 4. Numerical outputs — numerical gate

Float-heavy outputs are evaluated through:

- dimensions and units;
- sign and normalization contracts;
- analytic/known limits;
- residuals and conservation laws;
- declared absolute/relative tolerances;
- stability under tolerance/grid variation;
- observable parity.

`+0.0` versus `-0.0`, equivalent float text, or sub-tolerance low-bit drift is
not a failure unless the sign bit or exact bytes carry declared physical
semantics.

## 5. Packaging transport — soft gate

Tar/zip/wheel outer hashes are advisory unless the archive bytes themselves are
the release product. Content-equivalent repacks may differ because of file
ordering, mtime, uid/gid, permissions, compressor version, or header metadata.

For ordinary handoff/backup, verify safe extraction, required paths, Git-tracked
inner payloads, and content inventory. Record outer SHA mismatch as provenance,
not as a scientific blocker.

## Severity policy

- **P0:** wrong science, wrong immutable source/input, unauthorized ref/history
  mutation, hidden extra diff, test suppression, or success after partial
  verification.
- **P1:** unresolved specification or dependency closure, stale provenance
  binding, missing load-bearing test, or unexplained source materialization that
  would be committed/built.
- **P2:** reproducibility/documentation weakness without current scientific
  corruption.
- **P3/informational:** packaging hash mismatch with verified content
  equivalence and no byte-reproducibility claim.
