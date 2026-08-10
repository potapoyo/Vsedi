# 開発ロードマップ

## M0 — 製品定義

目的: 実装前に製品の境界と安全原則を固定する。

成果物:

- 製品ビジョン
- 対象ユーザー / 用語
- 設計原則
- 安全モデル
- MVP 要件 / 対象外項目
- アーキテクチャ判断記録（ADR）

完了条件:

- 「Vsedi が何をするか」と「何をしないか」をリポジトリ内の Markdown だけで説明できる
- Git backend / local-first / automatic merge 方針が ADR で決まっている

## M1 — Tauri 基盤

目的: Windows / macOS で共通のアプリ基盤と Rust command boundary を作る。

タスク:

- Tauri v2 プロジェクト初期化
- frontend stack の選定と初期化
- Rust ↔ frontend command boundary
- native folder picker
- Git executable detection
- Git version detection
- Git LFS detection
- structured / sanitized logging
- Windows build check
- macOS build check

完了条件:

- 両 OS でアプリが起動する
- project folder を選べる
- Rust 側から Git / Git LFS の存在を安全に確認できる

## M2 — Unity / VRChat プロジェクト検出

目的: Vsedi が管理対象 project を理解し、危険な状態を診断できる。

タスク:

- Unity project validation
- Unity version / project metadata reading
- VPM / VRChat project detection
- Avatar / World 判定の調査と可能な範囲での実装
- existing Git repository detection
- `.gitignore` diagnostics
- `.gitattributes` diagnostics
- VPM package source-control diagnostics
- large file diagnostics

完了条件:

- 選択 project について「管理可能 / 要修正 / 非 Unity」を説明できる

## M3 — ローカル保存

目的: 最初に実用になる Vsedi を作る。

タスク:

- Git repository initialization
- status parser
- changed-file model
- diff reading
- save memo UI
- add / commit
- history reading
- commit detail view

完了条件:

> Unity project を登録し「作業を保存」を押すと、その時点が履歴に残る。

この段階を Internal Alpha 候補とする。

## M4 — 安全な復元

目的: Vsedi の中心価値である「壊しても戻れる」を成立させる。

タスク:

- revision selection
- restore preview
- safety snapshot design ADR
- safety snapshot implementation
- restore operation
- restore validation
- return-to-pre-restore flow
- Unity-running warning

完了条件:

> 保存後に project を変更し、履歴から過去状態へ戻し、さらに復元前状態へも戻れる。

M0〜M4 を **Vsedi Core** とする。

## M5 — リモートバックアップ

目的: local-first の価値を保ったまま remote backup を追加する。

タスク:

- clone
- remote configuration
- fetch
- push
- fast-forward sync
- divergence detection
- Git credential helper integration
- authentication/error UX

明示的な制約:

- diverged history を自動 merge / rebase しない

## M6 — VRChat 向け安全機能

目的: 一般 Git GUI ではなく VRChat 専用ツールとしての診断を強化する。

タスク:

- VPM ignore rules
- VRChat SDK / package accidental commit detection
- `.meta` consistency diagnostics
- LFS recommendations
- large binary warnings
- SDK / package update checkpoint guidance
- Unity version update checkpoint guidance
- public repository / purchased asset warning design

## M7 — 初回チュートリアル

目的: Git を知らない新規ユーザーが最初の保存まで到達できる。

タスク:

- introduction
- project selection
- safety diagnostics walkthrough
- first save walkthrough
- local save vs remote backup explanation
- history walkthrough

完了条件:

- tutorial を通じて実際の最初の commit が作成される

## M8 — 公開ベータ

目的: 第三者へ配布できる品質へする。

タスク:

- Windows installer
- macOS app bundle
- code signing / notarization strategy
- updater strategy
- CI builds
- automated tests
- crash/error reporting policy
- log export / redaction
- About / GPL notices
- release documentation

## 後回しにするもの

明示的に後回しにする。

- automatic merge / rebase
- rich branch management
- cherry-pick
- force push
- stash UI
- submodules / worktrees
- GitHub PR / Issue management
- UnityYAMLMerge automatic conflict resolution
- multi-user file locking

必要性が実利用から確認された時点で再評価する。
