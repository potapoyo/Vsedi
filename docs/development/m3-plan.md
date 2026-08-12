# M3 再計画 — ローカル保存と Internal Alpha

Status: In Progress

Replanned: 2026-08-12

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

2026-08-12に追加したWindows native UI workflowは、release executableのbuildとWebView2 / Edge WebDriverの準備までは成功したが、WebDriverの`POST /session`で`DevToolsActivePort file doesn't exist`となり、テスト本体を1件も開始できなかった。実行対象をcapabilityとserviceの両方へ明示した後も再現したため、native UI CIの調査は一旦中断している。

この失敗はM3のRust / React実装に対するテスト失敗ではなく、Windows runner上でnative WebDriver sessionを作成できないテスト基盤の問題として扱う。ただし、native testが成功したことにもならないため、Windows native確認はInternal Alphaの未完了条件として残す。

検証経路は混同せず、次の4層に分ける。

| 層 | 役割 | 現在の状態 |
| --- | --- | --- |
| 通常CI | typecheck、frontend build、Rust test、Clippy、生成型整合性 | 利用可能 |
| Playwright smoke | IPCをmockした主要画面とnavigationの確認 | 手動workflowあり。新UI用ケース更新が必要 |
| Native UI test | WebView、Rust IPC、native windowを通る自動確認 | Windowsでsession作成に失敗し調査中断。macOSは未追加 |
| 配布物smoke | `.app` / DMG、Windows app / installerの実機確認 | macOSは旧UI時点で確認済み。Windowsと新UI再確認が必要 |

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
| 管理Project・カテゴリ | 実装済み | schema 4 migration、最終更新順、カテゴリ設定・絞り込みを実装済み |
| stale Project管理 | 実装済み・未実機確認 | 場所の再指定、一覧からの削除、重複登録防止を実装 |
| repository固有設定 | 実装済み・未実機確認 | VPM overrideをsettings schema 5へ保存し、診断・初期化で実効値を使用 |
| ignore template・差分適用 | 実装済み・未実機確認 | 全体設定で編集し、repository設定で不足ruleをpreviewして追加 |
| UI smoke更新 | 未着手 | 画面分割・管理Project・カテゴリをテストケースへ反映する |
| Windows native検証 | 中断中 | WebDriver session作成問題を解消後に再開する |
| macOS最終native検証 | 未完了 | 旧UIの保存導線・DMGは確認済み。現行UIで再確認する |

## 対象範囲

### M3に含める

- Git repositoryの初期化previewと実行
- ユーザーが編集可能なUnity / VPM ignore templateと、不足ruleのpreview・追記
- repository全体の変更状態とfile diffの読み取り
- 保存メモ、`add`、`commit`、保存直前の再検証
- commit履歴、commit詳細、変更file、表示可能なdiff
- 管理Project一覧、最終更新順、カテゴリ設定と絞り込み
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

### Phase A — 現在の製品差分を確定する

- 管理Project一覧、最終更新順、カテゴリ編集・解除・絞り込みをreviewする
- schema 1〜3からschema 4へのmigrationと、破損・未来schemaの拒否を再確認する
- stale pathの表示と、カテゴリ操作時のエラー表示を確認する
- 通常CI相当の検証を行い、この機能を単独のcommitとして確定する

完了条件: 既存設定を失わずに全Projectを一覧でき、カテゴリが再起動後も保持される。

### Phase B — 設定画面の未完部分を実装する

VPM tracking policyのrepository override、schema migration、ignore template編集・差分適用は実装済み。Mac実機確認は端末ロック中のため未実施とし、ローカル自動検証で先行確認する。

- repository固有のVPM追跡方針を「全体設定に従う / 除外する / 含める」で保存する
- 実効値と、全体既定値・repository overrideのどちらが採用されたかを表示する
- 全体設定でignore templateを編集し、リポジトリ設定から現在のignoreとの差分previewと不足ruleの明示適用を行えるようにする
- repository設定はapp dataの`settings.json`へ保存し、設定変更だけでrepositoryをdirtyにしない
- schema migration、canonical path、stale repository設定をテストする

完了条件: 全体設定とrepository設定の責務がUIと保存形式の両方で一致し、設定画面を開いただけではrepositoryを変更しない。

### Phase C — UI smoke testを現行画面へ追従させる

- `origin/main`のUI test基盤を、未コミット作業を確定した後にM3 branchへ取り込む
- JSONテストケースを、ホーム、Project選択、現在の作業、保存履歴、両設定画面、カテゴリ絞り込みへ更新する
- Rust IPCをmockし、成功・empty・blocking errorを決定的に再現する
- WindowsとApple Silicon macOSのPlaywright手動workflowを実行し、スクリーンショットとtraceをartifactで確認する
- UI workflowは当面`workflow_dispatch`のみとし、通常CIへは組み込まない

完了条件: 両OSのPlaywright smokeが現行UIの主要navigationと表示を通過する。

### Phase D — 製品機能の回帰検証を完了する

- 一時repositoryを使うRust統合テストでinit→status→save→history→detailを通す
- project rootと親repositoryの両構成を確認する
- 空白・日本語path、rename、削除、binary、大量diff、empty repositoryを確認する
- 設定migration、管理Project順、カテゴリ、repository overrideを含める
- typecheck、production build、Rust test、Clippy、生成型差分チェックを成功させる

完了条件: M3の状態変更と設定migrationに既知のデータ損失経路がなく、通常CIが成功する。

### Phase E — Native UI CIを再開する

製品実装と通常CIを止めず、手動workflow内で調査する。

1. Windows runnerでTauri executable単体の起動可否、process終了、WebView2 user-data directoryをartifactへ記録する
2. `tauri:options.webviewOptions`を明示した最小sessionを検証する
3. `@wdio/tauri-service`のembedded / external providerの現行対応範囲を確認する
4. 必要ならWebdriverIO v7 + 手動`tauri-driver`方式を比較用branchで試し、製品コードへテスト専用変更を入れない
5. session作成後に、起動、Rust環境診断、Project選択、ログwindowの最小ケースを通す
6. Windows方式が安定してから、macOS native workflowの方式を決める

完了条件: native test本体が少なくとも1件実行され、失敗時にdriver / app logとスクリーンショットを回収できる。安定するまでは通常CIの必須checkにしない。

### Phase F — 両OSの配布物smokeとInternal Alpha判定

- Apple Silicon macOSで現行UIの`.app` / DMGを起動し、init→save→history→detailを確認する
- Windowsでnative app / installerを起動し、同じ中心導線を確認する
- native UI自動化で未検証のOS統合は手動チェックリストで補い、結果を日記へ記録する
- README、architecture、roadmap、既知制約を実装結果へ更新する

完了条件: 「Unity projectを登録し『作業を保存』を押すと、その時点が履歴に残る」を両OSの配布物で再現できる。

## テストマトリクス

| 領域 | 最低限のケース |
| --- | --- |
| status parser | untracked、modified、deleted、rename、type change、conflict、空白・日本語path、NUL区切り |
| repository境界 | 未初期化、project=root、親repository、壊れた`.git`、別worktree |
| ignore | fileなし、既存ruleあり、不足rule、CRLF / LF、末尾改行なし、読取不能、編集済みtemplate |
| save | 初回commit、通常commit、変更なし、空memo、状態変化、既存staged、conflict、add失敗、commit失敗 |
| history | 初回commit、複数commit、rename、削除、merge commit、binary、履歴なし |
| settings | schema 1〜5、未来schema、破損JSON、カテゴリ、全体既定値、repository override、stale path |
| UI | loading、empty、blocking error、成功、再読込、二重click、長いpath / memo、カテゴリ絞り込み |
| platform | Playwright Windows / macOS、Apple Silicon `.app` / DMG、Windows native app / installer |

## M3完了条件

- 通常CIの全項目が成功する
- Frontendから任意のGit / shell commandを実行できない
- 保存前に対象変更が表示され、表示後に状態が変わった場合はcommitしない
- 失敗時にworktreeを自動で巻き戻さず、変更可能性と次の行動を説明できる
- 保存成功後にcommit IDと時刻が表示され、同じcommitを履歴と詳細画面から確認できる
- 管理Project、カテゴリ、全体設定、repository設定が再起動後も安全に復元される
- Windows / macOSのPlaywright smokeが現行画面で成功する
- macOSとWindowsの配布物でinit→save→history→detailのnative smokeに合格する
- native UI CIが未安定の場合、その事実を既知制約として残し、手動native smokeでInternal Alpha判定を補完する

## 実行順序

1. Phase A: 管理Project・カテゴリ差分を確定
2. Phase B: repository固有設定を完成
3. Phase C: UI test基盤を取り込み、現行画面へ追従
4. Phase D: M3全体の自動回帰検証
5. Phase E: Windows native UI CIの中断点から再開
6. Phase F: 両OSの配布物smokeとInternal Alpha判定

Windows native CIの調査はPhase A〜Dを妨げない。driver調査が長期化した場合も、製品実装・通常CI・Playwright・手動native smokeの結果を個別に記録し、未確認項目を成功扱いしない。
