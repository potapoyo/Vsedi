# MVP 要件

## 対象範囲

MVP は「Unity プロジェクトを登録し、安全に作業を保存し、履歴を確認し、現在状態を失わずに過去へ戻せる」ことを完成条件とする。

リモート同期は Vsedi Core 完成後に追加する。

## 製品としての必須要件

以下は特定マイルストーンだけの目標ではなく、Vsedi が配布可能な製品として満たす必須条件とする。

- デスクトップアプリ基盤として **Tauri v2** を使用する
- **Windows と macOS の両方を正式対応**する
- macOS は **Apple Silicon（arm64）のみ**を正式対応とする
- Windows / macOS は、それぞれ用意したネイティブ環境でビルド・検証できること
- エンドユーザーへ、ソースコードや開発環境ではなく **インストール可能なバイナリ**を配布できること
- エンドユーザーは Rust / Node.js / pnpm 等の開発ツールチェーンを導入せずに Vsedi をインストール・起動できること
- Windows では Tauri が生成するインストーラー形式（NSIS `.exe` または MSI `.msi`）を少なくとも1種類提供すること
- macOS では Apple Silicon 向け `.app` を含む `.dmg` を提供すること
- インストーラーバイナリの生成をリリース工程の正式な一部として扱うこと
- PC の初期化・故障・買い替え後に、必要な外部ツール、持ち運び可能な Vsedi 環境バックアップ、アクセス可能なリモートリポジトリから制作環境を再構成できる復元導線を提供すること

Git は ADR 0001 に従ってシステム Git CLI を利用するため、Git 自体の導入要件や未導入時の案内はアプリ側で別途扱う。

## 機能要件

### プロジェクト登録

- ユーザーが project directory を選択できる
- Unity project として妥当か検査できる
- VRChat / VPM project の可能性を診断できる
- Avatar SDK と Worlds SDK が同居する project は非対応エラーとして停止できる
- Git repository の有無を検出できる
- 管理している project を最終更新順で一覧できる
- project ごとに複数のアプリ内タグを設定し、タグで一覧を絞り込める
- Project名、project path、タグを対象に管理Projectを検索できる
- 管理Project一覧で、Avatarは人物アイコン、Worldは地球儀アイコンを表示し、マウスオーバーで種別を確認できる
- project folder を Finder / Explorer で開ける
- Unity で開く導線を提供できる

### 環境診断

- system Git の存在を検出できる
- Git version を表示できる
- `.gitignore` の状態を診断できる
- VPM packageをGit管理から除外するか含めるかを設定できる
- repositoryごとにVPM package追跡方針を上書きし、実効値の由来を確認できる
- 選択したVPM追跡方針からの明らかな逸脱を警告できる
- Git repository rootがUnity project外にある状態を正常な構成として情報表示できる
- project 設定ファイルを読み取れない場合に警告し、読み取れる範囲の診断を継続できる

### リポジトリ初期化

- 未初期化 project で Git repository を作成できる
- Unity / VRChat 向け ignore rule を提案できる
- 新規repository向けのUnity / VPM ignore templateを設定画面から編集できる
- 既存repositoryの不足ignore ruleを確認し、既存内容を削除せず追加できる
- 既存 `.gitignore` を無断で置換しない
- 必要な変更を preview してから適用できる

### 作業を保存

- worktree の変更を一覧できる
- worktree の変更一覧をユーザー操作で再読込できる
- 変更ファイルをrepositoryの相対pathによるツリーで表示し、変更種別・Git状態・ファイル種別・project外などの詳細を確認できる
- 「変更のみを表示」と「フォルダ内全体を表示」を切り替え、後者ではGit管理対象と無視されていない未管理ファイルを確認できる
- 詳細ツリーの列境界をマウスでドラッグし、長い名前や詳細が見えるよう表示幅を調整できる
- 「現在の作業」のproject・保存状態・診断サマリーを折りたたんで省スペース表示でき、異常時は自動的に展開する
- Project選択後の作業画面では重複する共通ヘッダーを省略し、選択中Projectと関連メニューを枠で識別できる
- 保存状態を色分けし、未保存のデータがある場合は注意を引く色と「未保存の変更あり」で表示する
- 新規 / 変更 / 削除を区別できる
- ユーザーが保存メモを入力できる
- 対象変更を commit として保存できる
- 保存中であることを表示し、Git CLI の stdout / stderr を実行中に確認できる
- 保存成功後に commit ID と時刻を確認できる

MVP では staging area の概念を通常 UI に露出させず、「今回の作業を保存する」単位で扱う。ただし実装上の Git index の挙動は仕様化・テストする。

### 保存履歴

- commit history を時系列で表示できる
- 保存履歴をFinderの詳細表示のような細い行・列で表示し、保存メモ、日時、commit IDを一覧で確認できる
- 保存履歴は20件程度ずつ読み込み、読み込んだ履歴より古い履歴がある場合は「さらに読み込む」で追加取得できる
- 保存履歴など項目数がウィンドウに収まらない画面は、サイドバーを維持したままメイン領域をウィンドウ内でスクロールできる
- 保存メモ、日時、commit ID を確認できる
- commit 間で変更されたファイルを表示できる
- 「保存の詳細」の変更ファイルを現在の変更と同じフォルダツリー・状態・詳細・種類の列で表示できる
- 保存詳細の列境界もマウスで調整できる
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

### 環境バックアップと復元モード

ADR 0007 に従い、PC を失った場合でも制作環境を再構成しやすい仕組みを提供する。

#### アプリ内部設定

- 初回チュートリアル完了状態、最近利用したプロジェクト、UI 設定等をローカルファイルへ保存できる
- 初期実装では Tauri Store を使用し、OS 標準のアプリデータ領域へ `settings.json` として保存する
- `settings.json` は Explorer / Finder 等から通常のファイルとしてコピーできること
- `settings.json` は整数の `schemaVersion` を必須とし、初期schemaは `1`、単一カテゴリ追加後は `4`、repository固有設定追加後は `5`、複数タグ移行後は `6`、OS生成ファイルのignore rule追加後の現行schemaは `7` とする
- 対応する `settings.json` を所定のアプリデータ領域へ手動配置した場合、Vsedi が通常の設定ファイルとして読み込めること
- 古い schema は可能な範囲で migration し、未対応 schema や破損 JSON を黙って上書きしないこと
- 手動復元された設定内の旧 PC の path が存在しない場合は、クラッシュせず再選択・再登録を促すこと
- アプリ内部設定は持ち運び可能な環境バックアップとは分離する

#### 持ち運び可能な環境バックアップ

- ユーザーが環境バックアップを `vsedi-environment.vsedi.json` として export / import できる
- backup format は `formatVersion` を必須とし、初期値は `1` とする
- Vsedi アプリバージョン、settings schema version、backup format version を分離する
- remote URL、通常利用する branch、Unity version、VCC / ALCOM 等の非秘密情報を復元に必要な範囲で保持できる
- 旧 PC の絶対 path は正本とせず、保存する場合も参考情報として扱う
- 新しい PC では project root / 復元先をユーザーが選択できる

#### 復元モード

- backup file を読み込み、形式を検証できる
- Git / Unity / VCC / ALCOM 等の必要環境を診断できる
- 不足している外部ツールをユーザーへ説明できる
- リモートリポジトリから clone できる
- Unity version / VPM 構成を診断できる
- VCC / ALCOM から利用可能な状態か診断できる
- 復元した project を Vsedi へ登録できる
- 復元完了時に不足項目や注意点を表示できる

初期の復元モードでは Unity / VCC / ALCOM 等の自動インストールまでは必須にしない。

#### 秘密情報

`settings.json` および持ち運び可能な環境バックアップには次を含めない。

- password
- GitHub Personal Access Token
- SSH private key
- Git credential helper が保持する秘密情報
- VRChat の認証情報
- VCC / ALCOM の秘密認証情報

必要な認証は復元先 PC で再設定する。

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

### 対応 OS・アーキテクチャ

- Windows を正式対象とする
- macOS は Apple Silicon（arm64）のみを正式対象とする
- Intel Mac（x86_64）は正式対応対象外とする
- OS 固有機能は adapter を分ける
- build / release validation は Windows と Apple Silicon Mac の各ネイティブ環境で行う

### バイナリ配布

- バイナリ配布は任意機能ではなく必須要件とする
- Windows と macOS のリリース成果物を再現可能な手順で生成できること
- Windows は一般ユーザーが実行可能なインストーラーを提供すること
- macOS は Apple Silicon 向け DMG を提供すること
- 開発者向けの `cargo tauri dev` 等を、一般ユーザーの導入手順として要求しない
- 配布成果物の最低限の起動確認を各対象 OS 上で行う

### コード署名・公証

- 初期配布は Windows / macOS とも未署名を正式方針とする
- 有料の証明書や開発者プログラムへの加入を、Vsedi のビルド・配布における必須条件にはしない
- 将来、無償で利用可能で運用上許容できるコード署名手段がある場合は再評価する
- Apple の Notarization（公証）は初期配布の必須要件にしない
- 未署名または公証なしの配布で OS の警告や追加操作が必要になる場合は、ユーザーへ明確な案内を提供する

### Rust / TypeScript 型共有

ADR 0008 に従う。

- Rust 側の共有データ型を正本（Source of Truth）とする
- `serde` で serialization 形式を定義する
- `ts-rs` で TypeScript 型を生成する
- Frontend 側で同一 DTO を手作業で二重定義しない
- Rust 型と生成済み TypeScript 型の不整合を CI / test で検出できる構成にする

### エラー処理

ADR 0009 に従う。

- Rust 側に共通 `AppError` と安定した `ErrorCode` enum を定義する
- `ErrorCode` は `<DOMAIN>_<CAUSE>` を基本とする `SCREAMING_SNAKE_CASE` とする
- Frontend は `error.code` で UI 分岐し、Git stderr / OS error message を文字列解析して判定しない
- `AppError` は少なくとも `code`、`message`、`technicalDetail`、`operation`、`mayHaveMutated` を表現できる
- mutation を伴う処理が途中失敗した場合、`mayHaveMutated` を保守的に設定する
- `AppError` / `ErrorCode` は Rust を正本として `serde + ts-rs` から TypeScript 型を生成する
- 未導入や利用不可などユーザーが次の行動を取れる状態は、可能な限りエラーではなく通常の診断結果として表現する
- 生の stderr / OS error をユーザー向け主要メッセージとして直接表示せず、必要な場合は redact 後の技術詳細として扱う

### セキュリティ

- Frontend に任意 command 実行能力を与えない
- Rust 側で Git operations を allowlist 的に実装する
- shell command を文字列結合して実行しない
- project path を各 mutation 前に検証する
- credentials を Vsedi 独自の平文設定へ保存しない
- diagnostic logs から secrets を除外する
- `settings.json` および持ち運び可能な環境バックアップへ秘密情報を含めない

### 信頼性

- Git command failure を成功扱いしない
- stdout / stderr / exit status を構造化して扱う
- mutation の途中失敗時にユーザーが現在状態を判断できる情報を残す
- filesystem / Git integration tests 用の temporary repository fixtures を用意する
- settings schema / backup format の旧 version を読み込む場合は migration または明確な非対応エラーを提供する

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
