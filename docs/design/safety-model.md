# 安全モデル

## 目的

Vsedi の安全モデルは「ユーザーの現在の作業状態を、本人が理解しないまま失わせない」ことを最優先とする。

## 操作の分類

### 安全な読み取り操作

原則として確認なしで実行できる。

- Git / Git LFS のバージョン検出
- リポジトリ状態の確認
- 履歴の読み取り
- diff の読み取り
- リモート状態の確認 / fetch
- Unity / VRChat プロジェクト診断

### 状態を変更するが復旧可能な操作

実行内容を UI で明示し、失敗時に状態を説明する。

- `git init`
- add / commit
- `.gitignore` / `.gitattributes` の追記・マージ
- remote 設定
- push

### 破壊的になり得る操作

通常フローでは事前に安全スナップショットを要求する。

- 過去状態への復元
- branch / revision の切り替えで worktree が変わる操作
- remote の状態をローカルへ反映する操作

## 安全スナップショット

過去状態へ戻す前に、現在の変更を失わないための保存点を作る。

基本フロー:

1. worktree の状態を検査する
2. 未保存変更があれば、復元前スナップショットを作成する
3. 作成された snapshot の commit ID を記録する
4. 対象 revision と変更内容をプレビューする
5. 復元を実行する
6. 復元結果を検証する
7. 問題があれば snapshot へ戻れる導線を表示する

snapshot の具体的な実装方法は Restore 実装前に追加 ADR で確定する。

## MVP で提供しない操作

次の操作は MVP の通常 UI では提供しない。

- force push
- `reset --hard`
- interactive rebase
- automatic rebase
- automatic merge conflict resolution
- branch history rewriting

## 履歴が分岐した場合

remote と local が分岐している場合、MVP は自動統合しない。

- fetch までは可能
- fast-forward 可能なら同期候補として表示
- fast-forward 不可能なら停止
- local / remote のそれぞれの先行 commit を表示
- 「Vsedi は安全のため自動統合しません」と説明する

## Unity を考慮した保護

### Unity 起動中

worktree を大きく変更する操作では Unity の起動状態を可能な範囲で検知する。

検知できた場合は Unity を閉じることを推奨し、復元・同期などでは強い警告を表示する。

### `.meta`

Unity asset と `.meta` の不整合が疑われる場合は警告する。Vsedi が独自判断で `.meta` を生成・削除しない。

### VPM パッケージ

VRChat 公式ガイドに従い、通常は `Assets/`, `ProjectSettings/`, `Packages/` の必要情報を追跡しつつ、VPM パッケージ本体は Resolver 等の例外を除いてソース管理から除外する。

### SDK / Unity 更新前のチェックポイント

SDK / VPM package / Unity 更新前には作業保存を促す。VRChat 公式も SDK 更新前の commit、および Unity 更新前の backup / version control を推奨している。

## 公開リポジトリへの警告

VRChat プロジェクトには再配布条件のある購入アセットが含まれる可能性がある。

Vsedi が将来 GitHub リポジトリ作成を支援する場合、公開リポジトリを選択する前に明示的な警告を出す。Vsedi がアセットの利用規約を自動判定できるとは表現しない。

## ログ

ログには次を極力含めない。

- password / token
- URL に埋め込まれた credential
- credential helper の応答

個人 path は診断上必要な場合があるため、ユーザーが共有するログを export する機能では redact を検討する。

## 参考資料

- VRChat VPM source control: https://vcc.docs.vrchat.com/vpm/source-control/
- VRChat SDK updates: https://creators.vrchat.com/sdk/updating-the-sdk/
- VRChat Unity upgrade guidance: https://creators.vrchat.com/sdk/upgrade/migrating-to-a-newer-minor-unity-version/
