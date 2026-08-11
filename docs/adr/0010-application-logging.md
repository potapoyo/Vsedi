# ADR 0010: アプリケーションログ

- 状態: 採用
- 日付: 2026-08-10

## 背景

Vsedi は Git、filesystem、Unity/VRChat プロジェクト等を扱うデスクトップアプリであり、ユーザー環境で発生した問題を安全に切り分けられるログが必要である。

一方で、Git remote URL、credential、token、password 等をログへ残すと、診断のためのログ自体が情報漏えい源になり得る。

## 決定

### 保存と保持

- ログは OS 標準の Vsedi 用ログ / アプリデータ領域へ保存する
- 日次でローテーションする
- ログは **30日間保持**する
- 古いログは保持期間を超えた時点で削除対象とする
- 将来、異常な肥大化を防ぐため容量上限を追加できる設計とする

### ログレベル

次の4段階を使用する。

- `ERROR`
- `WARN`
- `INFO`
- `DEBUG`

通常の配布版では `INFO` 以上を基本とする。`DEBUG` はユーザーが明示的に有効化した診断モード等でのみ使用する。

### 記録する情報

Git 等の外部 command については、必要に応じて次を記録できる。

- Vsedi 上の operation 名
- 結果 / exit status
- 所要時間
- 安定した `ErrorCode`
- `mayHaveMutated`
- sanitize 済みの診断情報

Git command の生の文字列をそのまま保存することを前提にしない。

### 秘密情報

次の情報はログへ記録しない。

- password
- Personal Access Token / access token
- credential を含む remote URL
- SSH private key の内容
- VRChat の認証情報
- VCC / ALCOM の秘密認証情報
- その他 secret と判断される値

remote URL を記録する必要がある場合は credential 部分を除去・匿名化する。

生の stdout / stderr に秘密情報が含まれる可能性を前提とし、そのままログへコピーしない。必要な技術情報は sanitize / redact 後に保存する。

### path の扱い

ローカルの project path は通常の端末内ログでは診断に必要な範囲で記録してよい。

ただし、ユーザーが外部共有用に「診断ログを書き出す」場合は、ホームディレクトリ名や OS ユーザー名等を含む path を匿名化する。

### UI

設定画面等から次の導線を提供できるようにする。

- 「ログフォルダを開く」
- 「診断ログを書き出す」

診断ログ export には、必要に応じて次を含める。

- 直近の関連ログ
- Vsedi version
- OS / architecture
- Git の診断結果
- 安全に共有可能なエラー情報

`settings.json` 本体や credential / secret は診断ログへ含めない。

## 影響

良い点:

- ユーザー環境の問題を30日間遡って調査しやすい
- ログを手動収集しなくても診断情報を共有しやすい
- secret がログへ混入するリスクを設計段階から抑えられる

注意点:

- sanitize / redact の実装とテストが必要になる
- DEBUG ログでも秘密情報禁止の原則は変わらない
- 診断ログ export 時には端末内ログより強い匿名化が必要になる

## 再検討する条件

- 30日保持でログ容量が実運用上問題になる場合
- 自動クラッシュレポートや telemetry を追加する場合
- 外部サポート向け診断 bundle の仕様を拡張する場合
