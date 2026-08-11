# ADR 0001: システムの Git CLI を使用する

- 状態: 採用
- 日付: 2026-08-10

## 背景

Vsedi は Windows / macOS 上で、通常のリポジトリ操作、認証、ユーザー既存の Git 環境との相互運用を含む Git 操作を行う必要がある。Git LFSを利用する既存repositoryとの互換性はsystem Gitの通常動作に委ね、Vsedi固有の診断・設定はADR 0012により行わない。

方式としては、Git 実装やライブラリをアプリへ組み込む方法と、システムにインストールされた Git executable を呼び出す方法が考えられる。

## 決定

Vsedi の初期 Git backend は、**Rust 側からシステムの Git CLI を呼び出す方式**とする。

Frontend code から任意の Git command を組み立てたり実行したりしない。Rust の application service が `get_status`、`save_work`、`get_history`、`restore_preview` などの明示的な操作を公開する。

command は shell command の文字列を連結するのではなく、executable と構造化された argument list を分けて実行する。

## 影響

利点:

- ユーザーのGit設定と通常のfilter integrationを尊重できる
- ユーザー既存の Git configuration を尊重できる
- credential helper や OS の secure store を再利用できる
- 上級者が terminal から確認できる通常の Git 挙動に近い
- 一般的な Git workflow との意味上の差異を減らせる

欠点:

- Git がインストールされている必要がある。将来 bundle / install を支援する場合は別途設計が必要
- output parsing を慎重に設計し、対応 Git version 間でテストする必要がある
- environment / PATH の差異へ対応する必要がある
- process execution は権限を伴う境界なので厳しく制限する必要がある

## セキュリティ上の制約

- Frontend へ任意 command execution を公開しない
- 通常の Git 操作で `sh -c`、`cmd /c`、PowerShell command string などを使用しない
- 状態変更操作の前に対象 project path を検証する
- exit status / stdout / stderr を取得する際に credential をログへ残さない

## 再検討する条件

システム Git が、導入性・可搬性・出力解析の面で許容できない問題を生むことが確認された場合にのみ、library backend を再検討する。

## 参考資料

- Git credentials: https://git-scm.com/docs/gitcredentials.html
- Tauri shell plugin: https://v2.tauri.app/plugin/shell/
