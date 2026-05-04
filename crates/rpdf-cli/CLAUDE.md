# rpdf-cli — CLI 바이너리 크레이트

## 역할

`rpdf` 바이너리 진입점. clap 서브커맨드로 PDF 조작 기능 노출.

## 현재 구현된 명령

```
rpdf info <file>            # 메타데이터 (버전, 페이지 수, 암호화 여부)
rpdf dump <file> [-p N]     # 페이지 N의 IR 덤프 (기본: 1페이지)
rpdf dump-pages <file>      # 전체 페이지 목록
```

## v0.2 추가 예정

```
rpdf render <file> -o <output.png>   # Task #12
rpdf render <file> --svg             # Task #14
```

## 파일 구조

- `src/main.rs` — clap App 정의
- `src/commands/info.rs` — info 서브커맨드
- `src/commands/dump.rs` — dump 서브커맨드
- `src/commands/dump_pages.rs` — dump-pages 서브커맨드

## 통합 테스트

`tests/cli_tests.rs` — `assert_cmd`로 바이너리 실행 검증

## 주의

- crate name `rpdf-cli`, binary name `rpdf` — Cargo.toml `[[bin]]` 섹션 확인
- `serde` 직접 임포트 필요 시 workspace `serde` 별도 선언 필요 (`serde_json`만으로 부족)
