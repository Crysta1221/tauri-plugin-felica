# tauri-plugin-felica

Sony PaSoRi を使い、Tauri v2 アプリから FeliCa の**無鍵領域**を読むプラグインです。

交通系 IC の残高・利用履歴、WAON / 楽天Edy / nanaco、QUICPay の識別、FeliCa Lite の S_PAD に対応します。暗号化領域の読み取りと、カードへの書き込みは行いません。

| 層 | パッケージ | 役割 |
| --- | --- | --- |
| Rust | `tauri-plugin-felica` | USB 通信、無鍵サービスの読み取りと数値解釈 |
| Webview | `tauri-plugin-felica-api` | `scan` / `poll` の型付き API と駅名・処理種別のラベル |

- 高レベル: `FelicaReader.scan()` → パース済み `ScanResult`
- 低レベル: `read()` および `poll()` → `FelicaCard`

対応リーダー: RC-S380 / RC-S300 / RC-S330 / RC-S360 / RC-S370 / RC-S320

使い方・導入・API は **[Wiki](https://github.com/Crysta1221/tauri-plugin-felica/wiki)** を参照してください。

## License

MIT OR Apache-2.0
