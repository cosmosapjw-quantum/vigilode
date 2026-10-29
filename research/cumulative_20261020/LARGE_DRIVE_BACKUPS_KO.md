# Google Drive 대형 오프라인 자원 백업

네 연구 ZIP은 모두 GitHub와 Drive 양쪽에 저장되어 있으므로 Drive 전용으로 남길 필요가 없다. 별도로 **큰 재생성 가능 자원**의 기존 Google Drive 백업도 확인했다.

## Rust 1.94.1 오프라인 배포본 (총 192,287,020바이트)

[part00 (64 MiB)](https://drive.google.com/file/d/1U4oLOF3sSR5VNTyHtpitw-0TOlxbtxIV/view) · [part01 (64 MiB)](https://drive.google.com/file/d/1fu5SQMGJ6BazIYOFl_afpoyeemEzof8T/view) · [part02 (58,069,292바이트)](https://drive.google.com/file/d/11xEVRh3ymvivu1TMBXq-YK_uoXMp8i9n/view)

동일한 세 이름으로 다운로드한 파일이 있는 디렉터리에서:

```bash
sha256sum -c - <<'SUMS'
9568ab151a1eea8c599ecc82f89c7ddf57edeb7aba1e81453ab5cb7afc3247df  rust-1.94.1-x86_64-unknown-linux-gnu.tar.xz.part00
44b831ea3f9cacddfd053e67d805537fbddd21749a2bb7477b784c561406ce06  rust-1.94.1-x86_64-unknown-linux-gnu.tar.xz.part01
0ae633ebc85e43dd1bc7a007be4dc628473fe3c79216b594c6573cdbf58a3123  rust-1.94.1-x86_64-unknown-linux-gnu.tar.xz.part02
SUMS
cat rust-1.94.1-x86_64-unknown-linux-gnu.tar.xz.part0{0,1,2} \
    > rust-1.94.1-x86_64-unknown-linux-gnu.tar.xz
echo '294b3d81fa72e62581276290c60c81eb8b58498d333d422ca1dfc432877d0c40  rust-1.94.1-x86_64-unknown-linux-gnu.tar.xz' | sha256sum -c
xz -t rust-1.94.1-x86_64-unknown-linux-gnu.tar.xz
```

이 SHA-256 값은 2026-10-20의 로컬 구성/복구 장부에 기록된 것이다. 본 누적 GitHub PR에서는 **바이너리 컴파일러 아카이브를 게시하지 않는다**. 네트워크가 있는 새 환경에서는 동등한 버전의 개발 toolchain을 `rustup toolchain install 1.94.1`로 설치할 수 있다. 다만 설치된 컴파일러 버전 일치가 성능 시간 재현을 보장하지 않는다.

## RUSTCORE 원본

[RUSTCORE(9).zip (33,103,685바이트)](https://drive.google.com/file/d/1S8x59K2B_KvRkGsGIfewv4hHl8yC95Yj/view)

SHA-256: `f30a4da90c68819d16df44632da91fa76e443df3b8dcf1d55c843bede1e1fb29`.

원본 연구 코드와 컴파일러 바이너리를 혼동하지 않는다. 이 오프라인 자원은 도구 재설치·환경 복원에 도움이 되는 보존물이며, 개별 GitHub 연구 커밋·원자료의 대체품은 아니다.

[원본 연구 ZIP 이중 백업 및 검증](README.md) · [복원 안내](REPRODUCE_ORIGINAL_ARCHIVES.md)
