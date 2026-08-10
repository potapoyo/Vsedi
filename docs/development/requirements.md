# MVP 要件

## 対象範囲

MVP は「Unity プロジェクトを登録し、安全に作業を保存し、履歴を確認し、現在状態を失わずに過去へ戻せる」ことを完成条件とする。

リモート同期は Vsedi Core 完成後に追加する。

## 機能要件

### プロジェクト登録

- ユーザーが project directory を選択できる
- Unity project として妥当か検査できる
- VRChat / VPM project の可能性を診断できる
- Git repository の有無を検出できる
- 最近利用した project を一覧できる
- project folder を Finder / Explorer で開ける
- Unity で開く導線を提供できる

### 環境診断

- system Git の存在を検出できる
- Git version を表示できる
- Git LFS の存在と version を検出できる
- `.gitignore` の状態を診断できる
- `.gitattributes` の状態を診断できる
- VPM のソース管理ルールからの明らかな逸脱を警告できる
- 大容量ファイル候補を検出できる

### リポジトリ初期化

- 未初期化 project で Git repository を作成できる
- Unity / VRChat 向け ignore rule を提案できる
- 既存 `.gitignore` を無断で置換しない
- 必要な変更を preview してから適用できる

### 作業を保存

- worktree の変更を一覧できる
- 新規 / 変更 / 削除を区別できる
- ユーザーが保存メモを入力できる
- 対象変更を commit として保存できる
- 保存成功後に commit ID と時刻を確認できる

MVP では staging area の概念を通常 UI に露出させず、「今回の作業を保存する」単位で扱う。ただし実装上の Git index の挙動は仕様化・テストする。

### 保存履歴

- commit history を時系列で表示できる
- 保存メモ、日時、commit ID を確認できる
- commit 間で変更されたファイルを表示できる
- 可能なテキストファイルでは diff を表示できる
- binary file は内容比較を無理に行わず、変更されたことを表示する

### 安全な復元

- 復元対象 revision を選択できる
- 復元によって変わるファイルを preview できる
- 現在状態に未保存変更がある場合は safety snapshot を作れる
- snapshot 作成が失敗した場合は復元を開始しない
- 復元完了後、復元前 snapshot へ戻る導線を提供できる

## Vsedi Core 完成後の要件

### リモートバックアップ

Core 完成後に追加する。

- existing remote の認識
- remote URL の設定
- clone
- fetch
- push
- fast-forward のみの pull / sync
- diverged history の検出と停止
- system Git credential helper の利用

### Git LFS 支援

- LFS 未導入警告
- LFS 対象候補の提案
- `.gitattributes` 変更 preview
- LFS push failure の検出

## 初回チュートリアル要件

初回チュートリアルは説明だけで終了させない。

1. Vsedi が何をするか説明する
2. project を選択する
3. Git / Unity / VRChat の安全診断を確認する
4. 最初の保存を実行する
5. commit は local 保存であり push とは異なることを説明する
6. 保存履歴を表示する

基本の完了条件は「最初の commit が作成されたこと」とする。

## 非機能要件

### 対応 OS

- Windows を正式対象とする
- macOS を正式対象とする
- OS 固有機能は adapter を分ける
- CI / release build は各 native platform で検証する

### セキュリティ

- Frontend に任意 command 実行能力を与えない
- Rust 側で Git operations を allowlist 的に実装する
- shell command を文字列結合して実行しない
- project path を各 mutation 前に検証する
- credentials を Vsedi 独自の平文設定へ保存しない
- diagnostic logs から secrets を除外する

### 信頼性

- Git command failure を成功扱いしない
- stdout / stderr / exit status を構造化して扱う
- mutation の途中失敗時にユーザーが現在状態を判断できる情報を残す
- filesystem / Git integration tests 用の temporary repository fixtures を用意する

### アクセシビリティと言語

- 初期 UI 言語は日本語を主対象とする
- Git 用語だけで操作を要求しない
- アイコンだけで危険操作の意味を表さない

## MVP で明示的に対象外とするもの

- rebase
- cherry-pick
- interactive rebase
- force push
- `reset --hard` UI
- stash UI
- submodule management
- worktree management
- advanced branch management
- Pull Request / Issue management
- GitHub Projects integration
- automatic UnityYAMLMerge conflict resolution
- multiplayer file locking
