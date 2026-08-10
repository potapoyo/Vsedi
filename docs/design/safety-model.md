# Safety Model

## Goal

Vsedi の安全モデルは「ユーザーの現在の作業状態を、本人が理解しないまま失わせない」ことを最優先とする。

## Operation classes

### Safe read operations

原則として確認なしで実行できる。

- Git / Git LFS version detection
- repository status
- history reading
- diff reading
- remote status / fetch
- Unity / VRChat project diagnostics

### Mutating but recoverable operations

実行内容を UI で明示し、失敗時に状態を説明する。

- `git init`
- add / commit
- `.gitignore` / `.gitattributes` の追記・マージ
- remote configuration
- push

### Potentially destructive operations

通常フローでは事前に safety snapshot を要求する。

- 過去状態への復元
- branch / revision の切り替えで worktree が変わる操作
- remote の状態をローカルへ反映する操作

## Safety snapshot

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

## Prohibited in MVP

次の操作は MVP の通常 UI では提供しない。

- force push
- `reset --hard`
- interactive rebase
- automatic rebase
- automatic merge conflict resolution
- branch history rewriting

## Diverged history

remote と local が分岐している場合、MVP は自動統合しない。

- fetch までは可能
- fast-forward 可能なら同期候補として表示
- fast-forward 不可能なら停止
- local / remote のそれぞれの先行 commit を表示
- 「Vsedi は安全のため自動統合しません」と説明する

## Unity-aware protections

### Unity running

worktree を大きく変更する操作では Unity の起動状態を可能な範囲で検知する。

検知できた場合は Unity を閉じることを推奨し、復元・同期などでは強い警告を表示する。

### `.meta`

Unity asset と `.meta` の不整合が疑われる場合は警告する。Vsedi が独自判断で `.meta` を生成・削除しない。

### VPM packages

VRChat 公式 guidance に従い、通常は `Assets/`, `ProjectSettings/`, `Packages/` の必要情報を追跡しつつ、VPM package 本体は Resolver 等の例外を除いて source control から除外する。

### SDK / Unity update checkpoints

SDK / VPM package / Unity 更新前には作業保存を促す。VRChat 公式も SDK 更新前の commit、および Unity 更新前の backup / version control を推奨している。

## Public repository warning

VRChat プロジェクトには再配布条件のある購入アセットが含まれる可能性がある。

Vsedi が将来 GitHub repository 作成を支援する場合、公開 repository を選択する前に明示的な警告を出す。Vsedi がアセットの利用規約を自動判定できるとは表現しない。

## Logging

ログには次を極力含めない。

- password / token
- URL に埋め込まれた credential
- credential helper の応答

個人 path は診断上必要な場合があるため、ユーザーが共有するログを export する機能では redact を検討する。

## References

- VRChat VPM source control: https://vcc.docs.vrchat.com/vpm/source-control/
- VRChat SDK updates: https://creators.vrchat.com/sdk/updating-the-sdk/
- VRChat Unity upgrade guidance: https://creators.vrchat.com/sdk/upgrade/migrating-to-a-newer-minor-unity-version/
