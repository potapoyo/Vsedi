# ADR 0005: 対応 OS とバイナリ配布

- 状態: 採用
- 日付: 2026-08-10

## 背景

Vsedi は VRChat 制作者が日常的に利用するデスクトップアプリであり、開発環境を用意できるユーザーだけを対象にはしない。

また、Windows と macOS の両方で利用できることが製品要件である。アプリ基盤には、両 OS 向けのネイティブデスクトップアプリとインストール用成果物を生成できる Tauri v2 を採用する。

macOS については、正式対応範囲を Apple Silicon に限定する。Intel Mac は正式対応対象外とする。

## 決定

Vsedi の正式対応環境と配布方式を次のようにする。

### Windows

- 正式対応 OS とする
- Tauri v2 でネイティブアプリをビルドする
- Windows のネイティブビルド環境で検証する
- 一般ユーザー向けに NSIS `.exe` または MSI `.msi` のインストーラーを少なくとも1種類提供する

### macOS

- 正式対応 OS とする
- **Apple Silicon（arm64）のみ**を正式対応する
- Intel Mac（x86_64）は正式対応しない
- Apple Silicon Mac のネイティブビルド環境で検証する
- 一般ユーザー向けに `.app` を含む `.dmg` を提供する

### 共通

- Tauri v2 をデスクトップアプリ基盤として使用する
- 一般ユーザーに Rust / Node.js / package manager 等の開発ツールチェーンを要求しない
- バイナリ配布は追加機能ではなく製品の必須要件とする
- Windows / macOS の成果物は各ネイティブ環境でビルド・検証する

## コード署名について

**初期配布は Windows / macOS とも未署名とする。**

コード署名・macOS Notarization は、初期リリースの必須条件にはしない。

そのため、未署名・未公証の配布物で Windows SmartScreen や macOS Gatekeeper の警告、追加操作が発生する場合は、ユーザーへ明確な案内を提供する。

将来、無償で利用可能で運用上許容できる署名手段が利用可能になった場合、または利用者規模・配布方式の変化により署名の必要性が高まった場合に再評価する。

## 理由

- Windows / macOS の両対応は製品の前提であり、後から変更すると設計・テスト・配布工程への影響が大きい
- Tauri v2 は両 OS 向けのデスクトップアプリと配布成果物を生成できる
- 一般ユーザーに開発環境を要求すると、初心者向けという Vsedi の製品方針と矛盾する
- Apple Silicon のみに限定することで macOS のビルド・テスト範囲を明確にできる
- 初期段階ではコード署名コストや証明書運用を必須にせず、まず利用可能なバイナリ配布を成立させる

## 影響

良い点:

- 対応環境とリリース成果物が明確になる
- 各 OS のネイティブ環境で確実に検証できる
- 一般ユーザーがソースからビルドする必要がない
- macOS の対応範囲を Apple Silicon に絞れる

注意点:

- 未署名・未公証の macOS アプリでは Gatekeeper の警告が発生し得る
- 未署名 Windows アプリでは SmartScreen の警告が発生し得る
- 将来、警告のない一般配布を重視する場合は署名方式を再検討する必要がある

## 再検討する条件

- ユーザーから Intel Mac 対応への十分な需要が確認された場合
- 無償または許容可能な費用で信頼済みコード署名を提供できるようになった場合
- App Store / Microsoft Store での配布を行う場合

## 参考資料

- Tauri Distribution: https://v2.tauri.app/distribute/
- Tauri macOS Code Signing: https://v2.tauri.app/distribute/sign/macos/
- Tauri Windows Code Signing: https://v2.tauri.app/distribute/sign/windows/
