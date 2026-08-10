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

有料のコード署名証明書や開発者プログラムへの加入は、Vsedi をビルド・配布するための必須条件にはしない。

### macOS

Tauri / macOS が対応するアドホック署名は無償で利用できるため、Developer ID を使用しない段階ではこれを利用可能な構成とする。

ただし、Apple の Notarization（公証）には有料 Apple Developer Program が必要であり、公証なしのアプリでは Gatekeeper による警告やユーザー側の追加操作が発生する可能性がある。

このため、無償配布段階では公証を必須にせず、必要な起動手順を明確に案内する。

### Windows

Windows の信頼済みコード署名は、一般に証明書または署名サービスを必要とする。Vsedi では有料証明書の購入を必須にしない。

公開ベータ前に、オープンソース向けなど無償で利用可能な信頼できる署名手段が採用可能かを再評価する。適切な無償手段が利用できない場合は、未署名インストーラーを配布し、SmartScreen 等の警告について明確に案内する。

## 理由

- Windows / macOS の両対応は製品の前提であり、後から変更すると設計・テスト・配布工程への影響が大きい
- Tauri v2 は両 OS 向けのデスクトップアプリと配布成果物を生成できる
- 一般ユーザーに開発環境を要求すると、初心者向けという Vsedi の製品方針と矛盾する
- Apple Silicon のみに限定することで macOS のビルド・テスト範囲を明確にできる
- 署名コストを製品開発の必須コストにせず、まず利用可能なバイナリ配布を成立させる

## 影響

良い点:

- 対応環境とリリース成果物が明確になる
- 各 OS のネイティブ環境で確実に検証できる
- 一般ユーザーがソースからビルドする必要がない
- macOS の対応範囲を Apple Silicon に絞れる

注意点:

- 公証なしの macOS アプリでは Gatekeeper の警告が発生し得る
- 未署名 Windows アプリでは SmartScreen の警告が発生し得る
- 将来、警告のない一般配布を重視する場合は署名コストを再検討する必要がある

## 再検討する条件

- ユーザーから Intel Mac 対応への十分な需要が確認された場合
- 無償または許容可能な費用で信頼済みコード署名を提供できるようになった場合
- App Store / Microsoft Store での配布を行う場合

## 参考資料

- Tauri Distribution: https://v2.tauri.app/distribute/
- Tauri macOS Code Signing: https://v2.tauri.app/distribute/sign/macos/
- Tauri Windows Code Signing: https://v2.tauri.app/distribute/sign/windows/
