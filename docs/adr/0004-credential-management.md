# ADR 0004: Git の認証情報管理を Credential Helper に委ねる

- 状態: 採用
- 日付: 2026-08-10

## 背景

リモート Git 操作では認証情報が必要になる場合がある。access token や password を Vsedi 独自の平文設定へ保存すると、不要なセキュリティリスクを生み、OS ごとの secure storage の仕組みを重複実装することになる。

Git には credential helper の仕組みがあり、OS の keychain や安全な credential store と連携する helper も利用できる。

## 決定

Vsedi は初期段階では独自の永続 Git credential store を実装しない。

システム Git CLI を通じて実行する remote operation では、ユーザーが設定している Git credential helper / askpass mechanism を利用する。

Vsedi は token や password を意図的に次へ書き込んではならない。

- application settings
- repository configuration
- diagnostic logs
- remote URLs

## 影響

利点:

- 実績のある Git authentication behavior を再利用できる
- 既存 helper を通じて OS secure storage と連携できる
- Vsedi 自体を password manager にする必要がない

欠点:

- ユーザーの Git installation / helper によって authentication UX が異なる場合がある
- 利用可能な helper / askpass setup がない環境では、GUI prompt integration を追加で設計する必要がある

## 将来の検討

将来 Vsedi が GitHub OAuth を正式に提供する場合は、別 ADR として設計する。その場合もこの判断を暗黙に置き換えず、OS backed secure storage を利用する。

## 参考資料

- https://git-scm.com/docs/gitcredentials.html
