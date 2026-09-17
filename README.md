# tauri-plugin-felica

Sony PaSoRi を使い、Tauri v2 アプリケーションから FeliCa カードの**無鍵領域**を読み取るプラグインです。

交通系 IC（Suica / PASMO / ICOCA 等）の残高・利用履歴、各種電子マネー（WAON / 楽天Edy / nanaco）、QUICPay の識別、FeliCa Lite / Lite-S のスクラッチパッド（S_PAD）の読み取りに対応しています。暗号化領域の読み取りやカードへの書き込みは行いません。

## 特徴

- **高レベル API (`FelicaReader.scan`)**: カードをかざすだけで自動認識し、残高や履歴をパース済みのオブジェクトとして取得
- **駅名・事業者名の自動解決**: サイバネ規格の駅コード・バス会社コードを組み込み辞書で自動変換
- **低レベル API (`poll` / `read`)**: 特定のサービスコードやブロックの直接読み取りにも対応
- **幅広いリーダー対応**: RC-S380 / RC-S300 / RC-S330 / RC-S360 / RC-S370 / RC-S320

## インストール

```bash
# フロントエンド (bun / npm / pnpm)
bun add tauri-plugin-felica-api
```

```toml
# src-tauri/Cargo.toml
[dependencies]
tauri-plugin-felica = { git = "https://github.com/Crysta1221/tauri-plugin-felica.git" }
```

## クイックスタート

```typescript
import { FelicaReader } from "tauri-plugin-felica-api";

// 1. リーダーに接続
const readers = await FelicaReader.list();
const reader = await FelicaReader.connect({ id: readers[0]?.id });

// 2. カードをスキャン (10秒待機)
const result = await reader.scan({ timeoutMs: 10000 });

// 3. 読み取り結果の利用
if (result.transit) {
  console.log(`交通系IC残高: ¥${result.transit.balance}`);
}

await reader.disconnect();
```

## ドキュメント (Wiki)

詳しい導入手順、チュートリアル、各カードのデータ仕様、トラブルシューティングは **[GitHub Wiki](https://github.com/Crysta1221/tauri-plugin-felica/wiki)** をご覧ください。

- [はじめに（導入ガイド）](https://github.com/Crysta1221/tauri-plugin-felica/wiki/Getting-Started)
- [チュートリアル：カードを読み取る](https://github.com/Crysta1221/tauri-plugin-felica/wiki/Tutorial-Scanning-Cards)
- [カード別読み取りガイド](https://github.com/Crysta1221/tauri-plugin-felica/wiki/Card-Guides)
- [低レベル API ガイド](https://github.com/Crysta1221/tauri-plugin-felica/wiki/Low-Level-API)
- [対応ハードウェアと環境](https://github.com/Crysta1221/tauri-plugin-felica/wiki/Supported-Hardware)
- [トラブルシューティング](https://github.com/Crysta1221/tauri-plugin-felica/wiki/Troubleshooting)

## License

MIT

