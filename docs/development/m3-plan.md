# M3 再計画 — ローカル保存と Internal Alpha

Status: In Progress

Replanned: 2026-08-12

Last updated: 2026-08-13

Reference: Issue #18

## 目的

M3では、診断済みのUnity / VRChat projectを登録し、Gitの用語を通常UIへ露出しすぎずに、現在の変更をローカルcommitとして保存し、保存履歴と変更内容を確認できる状態を作る。

完了時の中心導線は次のとおりとする。

1. ホームで管理Projectを追加または選択する
2. 未初期化なら、作成されるrepositoryとignore ruleの変更を確認して初期化する
3. 「現在の作業」で新規・変更・削除されたfileを確認する
4. 保存メモを入力して「作業を保存」を実行する
5. 「保存履歴」でcommit、変更file、表示可能なdiffを確認する
6. リポジトリ設定と全体設定を、それぞれの影響範囲に応じて変更する

この導線と配布物をApple Silicon macOSとWindowsで確認できた時点をInternal Alpha候補とする。

## 再計画の前提

2026-08-12に追加したWindows native UI workflowは、release executableのbuildとWebView2 / Edge WebDriverの準備までは成功したが、WebDriverの`POST /session`で`DevToolsActivePort file doesn't exist`となり、テスト本体を1件も開始できなかった。実行対象をcapabilityとserviceの両方へ明示した後も再現したため、native UI CIの調査を一旦中断した。2026-08-13に外部driverを使わないembedded WebDriver方式へ置き換え、Windows / macOSの両Actions jobで解消を確認した。

この失敗はM3のRust / React実装に対するテスト失敗ではなく、Windows runner上でnative WebDriver sessionを作成できない旧テスト基盤の問題として扱った。embedded方式でWindows native自動確認を完了し、Windows installerを使う手動配布物smokeも完了した。

検証経路は混同せず、次の4層に分ける。

| 層 | 役割 | 現在の状態 |
| --- | --- | --- |
| 通常CI | typecheck、frontend build、Rust test、Clippy、生成型整合性 | 利用可能 |
| Playwright smoke | IPCをmockした主要画面とnavigationの確認 | Windows / macOSの各4ケースがローカルとGitHub Actionsで成功。OS別artifactも取得済み |
| Native UI test | WebView、Rust IPC、native windowを通る自動確認 | embedded WebDriver方式でWindows / macOS Actionsが成功。実OS / System Gitのスクリーンショットを取得済み |
| 配布物smoke | `.app` / DMG、Windows app / installerの実機確認 | macOS現行UIの`.app`とApple Silicon DMG、Windows MSI / NSISの手動GUI smokeが完了 |

## 現在地

| 領域 | 状態 | 補足 |
| --- | --- | --- |
| repository境界・Git診断 | 完了 | project自身のrepositoryと親repositoryを区別する |
| 初期化preview | 完了 | 既存ignoreを置換せず、不足ruleだけを提示・追記する |
| status・worktree diff | 完了 | rename、binary、conflict、project外変更を扱う |
| 作業保存 | 完了 | TOCTOU、既存staged、conflict、二重実行を防止する |
| 保存履歴・commit詳細 | 完了 | text / binary / unavailable / truncatedを区別する |
| application log | 完了 | 5段階のlevelと保持期間内の全ログ表示を実装済み |
| 画面分割 | 完了 | ホーム、現在の作業、保存履歴、リポジトリ設定、全体設定 |
| 管理Project・タグ・検索 | 実装済み | schema 6 migration、最終更新順、複数タグ設定・タグ絞り込み・名前/path/tag検索を実装済み |
| stale Project管理 | 完了 | 場所の再指定、一覧からの削除、重複登録防止を実装・配布物で確認 |
| repository固有設定 | 完了 | VPM overrideをsettings schema 5からschema 6へ引き継ぎ、診断・初期化で実効値を使用・確認 |
| ignore template・差分適用 | 完了 | 全体設定で編集し、repository設定で不足ruleをpreviewして追加・確認 |
| UI smoke更新 | 完了 | 現行UIの4ケースをWindows / macOS fixtureで各4件成功。Actions run `31661021590`とartifactを確認済み |
| Windows native検証 | 完了 | embedded方式のActions run `31661030671`とWindows配布物手動GUI smokeが成功。履歴読み込み・表示用パス修正も反映済み |
| macOS最終native検証 | 完了 | embedded方式のActions、実機`.app`、Apple Silicon DMG同梱`.app`で起動、全体設定、実OS / System Git表示を確認済み |

## 他PC向け引き継ぎ時点の状態

最新の引き継ぎ基準は、リモートブランチ`codex/m3-local-save`の先頭とする。作業開始時は次で同期する。

```sh
git pull --ff-only origin codex/m3-local-save
```

M3の実装・テスト・Windows / macOS配布物確認は完了した。README、roadmap、Diary、別マシン向け手順の更新を反映し、次の開発はM4の安全な復元とする。

別マシンでの再開手順と既知の注意点は [`handoff.md`](handoff.md) にまとめている。

Windows native UI CIは旧external driver方式を廃止し、アプリ内のdebug限定embedded WebDriver方式へ更新した。Actionsの自動確認と配布物の手動GUI smokeは独立した結果として記録する。

## 対象範囲

### M3に含める

- Git repositoryの初期化previewと実行
- ユーザーが編集可能なUnity / VPM ignore templateと、不足ruleのpreview・追記
- repository全体の変更状態とfile diffの読み取り
- 保存メモ、`add`、`commit`、保存直前の再検証
- commit履歴、commit詳細、変更file、表示可能なdiff
- 管理Project一覧、最終更新順、複数タグ設定・タグ絞り込み・Project検索
- ホーム、現在の作業、保存履歴、リポジトリ設定、全体設定の分離
- 全体のVPM既定値とrepository固有override
- Rustの正本型からTypeScript bindingを生成する仕組み
- 通常CI、Playwright smoke、両OSのnative / bundle確認

### M3に含めない

- remote、push、pull、clone
- branchの作成・切替
- merge、rebase、conflict解消
- file単位の保存対象選択
- amend、履歴書換え、commit削除
- 過去状態への復元（M4）
- WebView描画待ち中の起動スプラッシュ（M4以降のUX改善）
- Git LFS診断
- UI testを毎commitで自動実行する運用

## 維持する安全条件

- Frontendへ任意のGit commandやshell commandを公開しない。用途別Tauri commandだけを使用する。
- 操作は正規化・検証済みのproject rootとrepository rootから開始する。
- 親folderがrepository rootの場合もrepository全体を保存対象とし、project外の変更をpreviewへ表示する。
- preview後、保存または初期化の直前に状態を再取得し、変化していれば処理を停止する。
- conflict、未完了のmerge / rebase、Git lock、読み取り不能file、既存staged変更がある場合は保存しない。
- `git add`後にcommitが失敗してもworktreeを巻き戻さず、indexが変化した可能性を表示する。
- 既存`.gitignore`を置換せず、元の改行形式を保って不足ruleだけを追記する。
- 空の変更、空白だけの保存メモ、同時保存、二重clickを拒否する。
- Gitのstdout / stderrやdiffを通常のapplication logへ記録しない。
- revisionはGitが返した完全なobject IDだけを受け付ける。

## 残工程

### Phase A — 現在の製品差分を確定する（実装・ローカル検証済み）

- 管理Project一覧、最終更新順、タグ編集・解除・絞り込み・検索を実装・review済み
- schema 1〜5からschema 6へのmigrationと、単一カテゴリからタグへの変換、破損・未来schemaの拒否を実装・テスト済み
- stale pathの表示、再指定、削除、重複登録防止を実装・配布物で確認済み
- 通常CI相当のRust test、Clippy、typecheck、production build、生成型差分チェックを複数回成功済み

完了条件: 既存設定を失わずに全Projectを一覧でき、タグが再起動後も保持される。

### Phase B — 設定画面とignore安全性を確認する

VPM tracking policyのrepository override、schema migration、ignore template編集・差分適用は実装・配布物確認済み。

- repository固有のVPM追跡方針を「全体設定に従う / 除外する / 含める」で保存する
- 実効値と、全体既定値・repository overrideのどちらが採用されたかを表示する
- 全体設定でignore templateを編集し、リポジトリ設定から現在のignoreとの差分previewと不足ruleの明示適用を行えるようにする
- repository設定はapp dataの`settings.json`へ保存し、設定変更だけでrepositoryをdirtyにしない
- schema migration、canonical path、stale repository設定をテストする

完了条件: 全体設定とrepository設定の責務がUIと保存形式の両方で一致し、設定画面を開いただけではrepositoryを変更しない。

### Phase C — UI smoke testを現行画面へ追従させる（完了）

- `origin/main`のUI test基盤をM3 branchへ取り込み済み
- JSONテストケースを、ホーム、Project選択、現在の作業、保存履歴、両設定画面、タグ編集・絞り込み・Project検索へ更新済み
- Rust IPCをmockし、成功・empty・blocking errorを決定的に再現する4ケースをローカルで成功済み
- WindowsとApple Silicon macOSで異なる実行環境fixtureを使い、ローカルで各4ケースを成功済み
- `pull_request`と`workflow_dispatch`でWindows / macOS matrixを実行し、結果とPlaywright artifactを保持するworkflowへ更新済み
- GitHub Actions run `31661021590`でmacOS 39秒、Windows 1分21秒で成功し、`ui-test-macos-1`と`ui-test-windows-1` artifactを確認済み

完了条件: 両OSのPlaywright smokeが現行UIの主要navigationと表示を通過する。

### Phase D — 製品機能の回帰検証を仕上げる

- 一時repositoryを使うRust統合テストでinit→status→save→history→detailを通す（基本flowは追加済み。全matrixの記録を仕上げる）
- project rootと親repositoryの両構成を確認する
- 空白・日本語path、rename、削除、binary、大量diff、empty repositoryを確認する
- 設定migration、管理Project順、タグ、Project検索、repository overrideを含める
- typecheck、production build、Rust test、Clippy、生成型差分チェックを成功させる

完了条件: M3の状態変更と設定migrationに既知のデータ損失経路がなく、通常CIが成功する。

### Phase E — Native UI CIを再開する（完了）

- `@wdio/tauri-service`のembedded providerを採用し、WindowsのEdgeDriverとmacOSの外部driverへの依存を廃止した
- `native-ui-test` Cargo feature、テスト専用Tauri設定、frontend pluginをdebug test buildだけへ分離した。通常buildではWDIOコード、権限、WebDriver serverを含めない
- Windows / macOS matrixでdebug appをbuildし、起動、ホーム、全体設定、実行環境、System Gitを確認する手動workflowを実装した
- 失敗時のスクリーンショットとWDIO / backend log、成功時の実行環境スクリーンショットをartifactへ保存する
- Apple Silicon macOS実機でnative UI smoke 1件が成功した
- GitHub Actions run `31661030671`でmacOS 2分35秒、Windows 5分37秒で成功した
- `native-ui-macos-1`と`native-ui-windows-1` artifactを取得し、成功スクリーンショットで`macos / aarch64`、`windows / x86_64`、System Gitが表示されることを確認した

完了条件: 達成。Actions上の両OSでnative test本体が1件成功し、成功スクリーンショットとWDIO / backend logを回収できた。当面は通常CIの必須checkにはせず手動workflowとして維持する。

### Phase F — 両OSの配布物smokeとInternal Alpha判定

- Apple Silicon macOSでは現行UIの`.app`でinit→save→history→detailを確認済み。現行UI DMGを生成し、読み取り専用マウント、同梱`.app`の起動、実行環境・System Git表示、終了、取り外しまで確認済み
- Windowsでは再ビルド、MSI / NSIS生成、インストール、環境診断、Project追加、初期化、保存、履歴・detail確認まで完了した。履歴解析とWindowsパス表示の修正も配布物へ反映済み
- native UI自動化で未検証のOS統合は手動チェックリストで補完済み
- README、roadmap、M3計画、別マシン向け引き継ぎ資料を実装結果へ更新済み

完了条件: 「Unity projectを登録し『作業を保存』を押すと、その時点が履歴に残る」を両OSの配布物で再現できる。

## テストマトリクス

| 領域 | 最低限のケース |
| --- | --- |
| status parser | untracked、modified、deleted、rename、type change、conflict、空白・日本語path、NUL区切り |
| repository境界 | 未初期化、project=root、親repository、壊れた`.git`、別worktree |
| ignore | fileなし、既存ruleあり、不足rule、CRLF / LF、末尾改行なし、読取不能、編集済みtemplate |
| save | 初回commit、通常commit、変更なし、空memo、状態変化、既存staged、conflict、add失敗、commit失敗 |
| history | 初回commit、複数commit、rename、削除、merge commit、binary、履歴なし |
| settings | schema 1〜6、未来schema、破損JSON、タグ、全体既定値、repository override、stale path |
| UI | loading、empty、blocking error、成功、再読込、二重click、長いpath / memo、タグ絞り込み、Project検索 |
| platform | Playwright Windows / macOS、Apple Silicon `.app` / DMG、Windows native app / installer |

## M3完了条件

- 通常CIの全項目が成功する
- Frontendから任意のGit / shell commandを実行できない
- 保存前に対象変更が表示され、表示後に状態が変わった場合はcommitしない
- 失敗時にworktreeを自動で巻き戻さず、変更可能性と次の行動を説明できる
- 保存成功後にcommit IDと時刻が表示され、同じcommitを履歴と詳細画面から確認できる
- 管理Project、タグ、全体設定、repository設定が再起動後も安全に復元される
- Windows / macOSのPlaywright smokeが現行画面で成功する
- macOSとWindowsの配布物でinit→save→history→detailのnative smokeに合格する
- native UI CIが未安定の場合、その事実を既知制約として残し、手動native smokeでInternal Alpha判定を補完する

## UX改善 — WebView起動スプラッシュ

DMG同梱アプリの確認時に確認された、プロセス起動直後の白画面を先に解消するため、起動スプラッシュを実装した。スプラッシュはReact画面とは独立した静的HTMLで、WebViewの初回描画待ちをブランド付きの画面として表示する。

### 実装済み

- `public/splash.html`にダミーアイコンと`Vsedi`のアプリ名を配置し、Tauriの`splashscreen`ウィンドウから読み込むようにした
- Tauriの`main`ウィンドウを初期状態で非表示にし、静的スプラッシュを先に表示する設定を追加した
- Reactの初回マウント後に`app-ready`イベントを一度だけ通知し、Rust側でメイン画面表示・フォーカス、スプラッシュ終了を行うready handshakeを追加した
- 通常のViteブラウザ実行ではTauriイベントブリッジがないため、イベント送信失敗を握りつぶして既存のPlaywright UI testと共存させた
- `cargo check`、`CI=true pnpm build`、`CI=true pnpm build:native-ui-test`で設定・型・Tauri debug buildの成立を確認した

### 残りの計画

1. Rust command、settings読み込み、store復元などの初期化失敗時は、無期限にスプラッシュを表示せず、再試行または診断情報を含むエラー画面へ遷移する。
2. Apple Silicon macOSの`.app` / DMG、Windowsのexe / installer、GitHub Actions native UI testで、低速起動・通常起動・初期化失敗の3状態を確認する。

### 完了条件

- WebView描画待ち中に白画面を表示せず、ブランド付きスプラッシュが表示される（実装・配布物での目視確認済み）
- React画面の描画完了後にスプラッシュが確実に消え、ホーム画面を操作できる（実装・配布物での目視確認済み）
- 初期化失敗時にスプラッシュが停止せず、ユーザーが再試行または診断へ進める
- 起動時間、ready handshake、失敗理由がログへ安全に記録され、機微情報を含めない

## CIを除外した実行順序

1. Phase A: 管理Project・タグ・検索差分を確定
2. Phase B: repository固有設定を完成
3. Phase D: 製品機能のローカル回帰検証を完成
4. Phase F: macOS現行UIのDMG配布物smokeを実施（完了）
5. Windowsが利用可能になった段階でinstaller配布物smokeを実施
6. README、Diary、既知制約を更新してInternal Alpha判定

Phase C（Playwright）とPhase E（native UI CI）は別工程として再開し、Windows / macOS Actionsで完了した。Windows installer配布物smokeと文書更新も完了し、M3の状態を別マシンへ引き継げる状態になっている。

Windows native CIのdriver調査はembedded方式への移行で完了した。今後も製品実装、通常CI、Playwright、native自動テスト、手動配布物smokeの結果を個別に記録し、未確認項目を成功扱いしない。
