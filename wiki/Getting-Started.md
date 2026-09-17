# はじめに（導入ガイド）

このガイドでは、Tauri v2 アプリケーションに `tauri-plugin-felica` を導入し、FeliCa カードの読み取りを開始するまでの手順を説明します。

---

## 前提条件

- **Tauri v2** プロジェクトがセットアップされていること
- **対応 OS**: Windows 10 / 11（PaSoRi USB デバイスへの直接アクセスに対応）
- **対応 PaSoRi リーダー**: RC-S380, RC-S300, RC-S330, RC-S360, RC-S370, RC-S320

---

## 1. パッケージのインストール

### Rust 側の依存関係追加

Tauri アプリケーションの `src-tauri/Cargo.toml` の `[dependencies]` に `tauri-plugin-felica` を追加します。

```toml
[dependencies]
tauri = { version = "^2.0.0" }
tauri-plugin-felica = { git = "https://github.com/Crysta1221/tauri-plugin-felica.git" } # または crates.io から
```

### フロントエンド側の依存関係追加

フロントエンドのパッケージマネージャーに合わせて、`tauri-plugin-felica-api` をインストールします。

```bash
# bun を利用する場合
bun add tauri-plugin-felica-api

# npm を利用する場合
npm install tauri-plugin-felica-api

# pnpm を利用する場合
pnpm add tauri-plugin-felica-api
```

---

## 2. プラグインの登録 (Rust)

`src-tauri/src/lib.rs`（または `main.rs`）の `tauri::Builder` でプラグインを初期化します。

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // felica プラグインを登録
        .plugin(tauri_plugin_felica::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## 3. パーミッション（権限）の設定

Tauri v2 のセキュリティモデルに従い、フロントエンドからプラグインの API を呼び出すための権限を付与します。

`src-tauri/capabilities/default.json`（または使用している capability 設定ファイル）の `permissions` 配列に `"felica:default"` を追加してください。

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "アプリのデフォルト権限",
  "windows": ["main"],
  "permissions": ["core:default", "felica:default"]
}
```

> [!TIP] `"felica:default"` には、リーダー一覧取得、接続、スキャン、個別ブロック読み取りなど、プラグインが提供するすべての標準コマンド実行権限が含まれています。

---

## 4. PaSoRi ドライバの準備（Windows）

本プラグインは USB 経由でリーダーと直接通信します。

- **RC-S380 / RC-S300** Sony 公式の「NFCポートソフトウェア」がインストールされている状態、または標準の WinUSB ドライバが適用されている状態で動作します。
- **診断ツールによる確認** Sony 公式の「NFCポート自己診断ツール」でリーダーが正常に認識されていることを確認してください。
- **排他制御について** PaSoRi は 1 つのプロセスからしか占有して通信できません。Sony の「SFCard Viewer」や「かざすリーダー」など、他の FeliCa 利用ソフトウェアが起動している場合は終了させておいてください。

---

次のステップ: **[チュートリアル：カードを読み取る](Tutorial-Scanning-Cards)** に進み、実際にカードを読み取ってみましょう。
