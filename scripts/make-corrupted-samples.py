#!/usr/bin/env python3
"""samples/ 손상 PDF 재생성 스크립트.

원본 PDF에서 의도적으로 손상된 B1, B2 파일을 생성한다.
원본: samples/trad-xref-basicapi.pdf (Apache 2.0, Mozilla pdf.js)
"""

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).parent.parent
SAMPLES = REPO_ROOT / "samples"
SOURCE = SAMPLES / "trad-xref-basicapi.pdf"


def make_b1_missing_trailer(source: bytes, outpath: Path) -> None:
    """B1: trailer 이후 전체 제거 → MissingEof / MissingTrailer"""
    trailer_pos = source.rfind(b"trailer")
    if trailer_pos < 0:
        print(f"ERROR: 'trailer' 키워드를 찾을 수 없음: {SOURCE}", file=sys.stderr)
        sys.exit(1)
    result = source[:trailer_pos]
    outpath.write_bytes(result)
    print(f"B1 생성: {outpath.name} ({len(result)} bytes) — trailer 이후 제거")


def make_b2_bad_xref_offset(source: bytes, outpath: Path) -> None:
    """B2: startxref 오프셋을 파일 범위 밖 값으로 오염 → xref 오프셋 오류"""
    data = bytearray(source)
    sxr = data.rfind(b"startxref")
    if sxr < 0:
        print(f"ERROR: 'startxref' 키워드를 찾을 수 없음: {SOURCE}", file=sys.stderr)
        sys.exit(1)

    # startxref 이후 공백 건너뛰기
    i = sxr + len(b"startxref")
    while i < len(data) and chr(data[i]) in " \r\n":
        i += 1

    # 오프셋 숫자 끝 위치 찾기
    j = i
    while j < len(data) and chr(data[j]).isdigit():
        j += 1

    original = data[i:j].decode()
    bad_offset = b"99999999"
    data[i:j] = bad_offset

    outpath.write_bytes(bytes(data))
    print(
        f"B2 생성: {outpath.name} ({len(data)} bytes) "
        f"— startxref {original} → {bad_offset.decode()}"
    )


def main() -> None:
    if not SOURCE.exists():
        print(f"ERROR: 원본 파일 없음: {SOURCE}", file=sys.stderr)
        print("  samples/ 디렉터리에서 실행하거나 trad-xref-basicapi.pdf를 확인하세요.")
        sys.exit(1)

    source = SOURCE.read_bytes()
    print(f"원본: {SOURCE.name} ({len(source)} bytes)")

    make_b1_missing_trailer(source, SAMPLES / "broken-missing-trailer.pdf")
    make_b2_bad_xref_offset(source, SAMPLES / "broken-bad-xref-offset.pdf")

    print("완료.")


if __name__ == "__main__":
    main()
