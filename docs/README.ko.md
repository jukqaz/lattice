# Lattice 문서

[English](README.md) | 한국어 | [Repository README](../README.ko.md)

이 directory는 public documentation과 LLM-oriented agent guidance를 분리합니다.
Public docs는 Lattice를 평가하거나 사용하는 사람의 주 진입점입니다.

## 추천 읽기 순서

1. [사용자 가이드](user/usage.ko.md): Lattice 설치, service 생성, 첫 backup,
   안전한 restore, Git sync.
2. [제품 범위](product/mvp-scope.ko.md): Lattice가 의도적으로 하는 것과 하지 않는 것,
   현재 release 범위.
3. [변경 로그](../CHANGELOG.ko.md): release별 behavior change와 migration note.
4. [Repository README](../README.ko.md): 빠른 command reference.

## Public Docs

| 우선순위 | 문서 | 용도 |
| --- | --- | --- |
| 1 | [사용자 가이드](user/usage.ko.md) | 첫 설정과 일반 작업 |
| 2 | [제품 범위](product/mvp-scope.ko.md) | 제품 경계와 release scope |
| 3 | [변경 로그](../CHANGELOG.ko.md) | upgrade와 release history |
| 4 | [English User Guide](user/usage.md) | English day-one setup |
| 5 | [English Product Scope](product/mvp-scope.md) | English product boundaries |
| 6 | [English Changelog](../CHANGELOG.md) | English release history |

## LLM Docs

LLM docs는 coding agent 실행 규칙이므로 English-only로 유지합니다.

| 문서 | 용도 |
| --- | --- |
| [LLM Documentation Index](llm/README.md) | Agent-facing repository guidance |
| [Branch And Release Policy](llm/branch-release-policy.md) | Commit, PR, CI, tag, release rules |

## Language Links

- English documentation index: [docs/README.md](README.md)
- English root README: [README.md](../README.md)
