# Vsedi

Vsedi は、VRChat 向け Unity プロジェクトの作業を安全に保存・確認・復元するためのデスクトップアプリです。

内部では Git を使用しますが、Git に詳しくない VRChat 制作者でも「作業を保存」「保存履歴を見る」「過去の状態に戻す」「リモートへバックアップする」といった操作を扱えることを目指します。

## 方針

- Windows / macOS 対応
- Tauri v2 を利用
- ローカルファースト（Local First）: GitHub 等のリモートがなくても価値が成立する
- 安全性を機能性より優先（Safety over Power）: 高機能な Git GUI より事故防止を優先する
- Unity / VRChat / VPM の構成を理解して安全性を高める
- 仕様・設計思想・重要な判断は `docs/` に Markdown で残す

## 現在の段階

製品定義と安全設計を固める M0 フェーズです。実装ロードマップは [`docs/development/roadmap.md`](docs/development/roadmap.md) を参照してください。

## ドキュメント

- [`docs/product/vision.md`](docs/product/vision.md) — 製品ビジョンと対象ユーザー
- [`docs/design/principles.md`](docs/design/principles.md) — 設計原則
- [`docs/design/safety-model.md`](docs/design/safety-model.md) — 復元・破壊的操作の安全モデル
- [`docs/design/architecture.md`](docs/design/architecture.md) — アーキテクチャと権限境界
- [`docs/development/requirements.md`](docs/development/requirements.md) — MVP 要件
- [`docs/development/roadmap.md`](docs/development/roadmap.md) — 開発フェーズ
- [`docs/adr/`](docs/adr/) — アーキテクチャ判断記録（ADR）

## ライセンス

GNU General Public License v3.0。詳細は [`LICENSE`](LICENSE) を参照してください。
