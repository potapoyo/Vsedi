# Vsedi

Vsedi は、VRChat 向け Unity プロジェクトの作業を安全に保存・確認・復元するためのデスクトップアプリです。

内部では Git を使用しますが、Git に詳しくない VRChat 制作者でも「作業を保存」「保存履歴を見る」「過去の状態に戻す」「リモートへバックアップする」といった操作を扱えることを目指します。

さらに、PC の故障・初期化・買い替え後に、Unity Editor、VCC または ALCOM、Git 等の必要環境、Vsedi の持ち運び可能な環境バックアップ、アクセス可能なリモートリポジトリを使って制作環境を再構成できる復元モードを目標とします。

## 方針

- Windows / Apple Silicon macOS 対応
- Tauri v2 を利用
- React + TypeScript + Vite + pnpm + Tailwind CSS + shadcn/ui を採用
- Rust / TypeScript 間の共有型は `serde + ts-rs` で Rust を正本として管理
- ローカルファースト（Local First）: GitHub 等のリモートがなくても価値が成立する
- 安全性を機能性より優先（Safety over Power）: 高機能な Git GUI より事故防止を優先する
- Unity / VRChat / VPM の構成を理解して安全性を高める
- アプリ内部設定と持ち運び可能な環境バックアップを分離する
- 環境バックアップへ password / token / SSH private key 等の秘密情報を含めない
- Windows / macOS 向けインストーラーバイナリの配布を必須とし、初期配布は未署名とする
- 仕様・設計思想・重要な判断は `docs/` に Markdown で残す

## 現在の段階

Tauri v2 / React の M1 基盤を実装中です。Rust 側の command boundary、Git / Git LFS 環境診断、設定保全、構造化ログ、生成型、最小 UI を含みます。実装ロードマップは [`docs/development/roadmap.md`](docs/development/roadmap.md) を参照してください。

## ドキュメント

- [`docs/product/vision.md`](docs/product/vision.md) — 製品ビジョンと対象ユーザー
- [`docs/design/principles.md`](docs/design/principles.md) — 設計原則
- [`docs/design/safety-model.md`](docs/design/safety-model.md) — 復元・破壊的操作の安全モデル
- [`docs/design/architecture.md`](docs/design/architecture.md) — アーキテクチャと権限境界
- [`docs/development/requirements.md`](docs/development/requirements.md) — MVP / 製品要件
- [`docs/development/roadmap.md`](docs/development/roadmap.md) — 開発フェーズ
- [`docs/adr/`](docs/adr/) — アーキテクチャ判断記録（ADR）

## 開発コマンド

```sh
pnpm install
pnpm tauri dev
pnpm tauri build
pnpm generate-types
pnpm check-generated-types
```

`pnpm tauri dev` / `pnpm tauri build` は Rust、Tauri の OS 依存ライブラリ、対象 OS の native toolchain が必要です。公式対応環境は Windows と Apple Silicon macOS です。

## 配布方針

- Windows: NSIS `.exe` または MSI `.msi` を少なくとも1種類提供
- macOS: Apple Silicon（arm64）向け `.dmg` を提供
- Intel Mac は正式対応対象外
- 初期配布は未署名とし、OS の警告や必要な起動手順を明確に案内する

## ライセンス

GNU General Public License v3.0。詳細は [`LICENSE`](LICENSE) を参照してください。
