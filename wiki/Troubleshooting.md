# トラブルシューティング

PaSoRi リーダーの接続やカードスキャン中に問題が発生した場合の対処方法をまとめています。

---

## エラーコード一覧 (`FelicaErrorCode`)

プラグインの操作でエラーが発生した場合、`FelicaError` がスローされます。`err.code` から原因を判別できます。

```typescript
import { FelicaError } from "tauri-plugin-felica-api";

try {
  await reader.scan({ timeoutMs: 5000 });
} catch (err) {
  if (err instanceof FelicaError) {
    switch (err.code) {
      case "DEVICE_NOT_FOUND":
        // リーダーが見つからない
        break;
      case "DEVICE_BUSY":
        // リーダーが他のプロセスで使用中
        break;
      case "DEVICE_ACCESS_DENIED":
        // USB アクセス権限エラー
        break;
      case "SCAN_TIMEOUT":
        // カード待機タイムアウト
        break;
      case "SCAN_CANCELLED":
        // ユーザーによる中断
        break;
      case "UNSUPPORTED_CARD":
        // 非対応カード（社員証など既知の無鍵プロファイルなし）
        break;
      case "CARD_TYPE_MISMATCH":
        // targets で指定した製品以外のカードがかざされた
        break;
    }
  }
}
```

| エラーコード | 主な原因 | 推奨される対処 |
| :--- | :--- | :--- |
| `DEVICE_NOT_FOUND` | PaSoRi が USB ポートに接続されていない、またはドライバが認識されていない | ケーブルの接続確認、PC本体の直接ポートへの差し替え |
| `DEVICE_BUSY` | 他のアプリケーションや別セッションが PaSoRi を排他制御している | 他の FeliCa 関連アプリを終了する |
| `DEVICE_ACCESS_DENIED` | USB インターフェース（WinUSB）のオープンに失敗した | 常駐ソフトの確認、ドライバ自己診断ツールの実行 |
| `DEVICE_DISCONNECTED` | スキャン中または通信中に USB ケーブルが抜かれた | 再接続してセッションを再確立する |
| `SCAN_TIMEOUT` | 指定した `timeoutMs` の間にカードがかざされなかった | 画面に「もう一度かざしてください」等の案内を表示して再試行 |
| `SCAN_CANCELLED` | `AbortSignal` または `cancelScan()` により中断された | 正常なキャンセル動作としてハンドリング |
| `UNSUPPORTED_CARD` | 既知の無鍵プロファイル（交通系、電子マネー、Lite等）を持たないカード | 社員証や学生証など、暗号化領域のみのカードでは無鍵データは読めません |
| `CARD_TYPE_MISMATCH` | `targets` で指定した製品以外のカードがかざされた | 該当のカード種別をかざすよう案内を表示 |
| `COMMUNICATION_ERROR` | カードとの RF 通信中に電波が途切れた（カードを早く離しすぎた等） | カードをしっかりリーダー中央にタッチしたままにする |
| `PROTOCOL_ERROR` | 想定外のフレームを受信した、またはスキャン中にカードが入れ替わった | カードを静止させた状態で再読み取り |
| `SESSION_CLOSED` | 切断済みまたは解放済みのリーダー/カードハンドルを操作した | 新たに `connect` または `poll` をやり直す |
| `INTERNAL_ERROR` | プラグイン内部またはシステムレベルの予期せぬエラー | ログの確認とリーダーの再接続 |

---

## よくあるトラブルと解決策

### 1. リーダーが認識されない (`DEVICE_NOT_FOUND`)

- **USB ポートの確認**
  USB ハブや延長ケーブルを経由している場合、給電不足や通信遅延により認識されないことがあります。PC 本体の USB ポート（可能であれば背面ポート）に直接接続してください。
- **Sony 公式診断ツールでの確認**
  Sony が提供している「NFCポート自己診断ツール」を起動し、リーダーが正常に認識されているかテストしてください。

### 2. リーダーが使用中・アクセス拒否される (`DEVICE_BUSY`, `DEVICE_ACCESS_DENIED`)

PaSoRi はハードウェアの仕様上、同時に複数のプロセスからアクセスすることができません。

- **常駐ソフトウェアの終了**
  以下のソフトウェアがバックグラウンドで起動している場合、リーダーが占有されてエラーになります。タスクトレイ等から終了してください。
  - Sony SFCard Viewer（SFカードビューワー）
  - 楽天Edy ビューアー / かざすフォルダー
  - e-Tax 関連のスマートカード常駐ツール
  - マイナポータルアプリ
- **RC-S300 をお使いの場合**
  Sony の「NFCポート自己診断ツール」で「RC-S300/P WinUSB」が○になっているか確認してください。

### 3. カードをタッチしても読み取れない (`COMMUNICATION_ERROR` / `UNSUPPORTED_CARD`)

- **カードを離すタイミング**
  FeliCa の読み取りには数十ミリ秒〜数百ミリ秒かかります。音が鳴る（または画面に表示が出る）まで、カードをリーダーの中央に静止させて置いてください。
- **スマートフォン（モバイルSuica 等）の場合**
  スマートフォンの FeliCa アンテナ位置（機種によって背面中央、カメラ付近など異なります）を PaSoRi のマークに合わせてください。厚手のスマホケースや金属製リングを装着していると電波が届かない場合があります。
- **複数枚のカードの重ねタッチ**
  パスケース等に複数の IC カード（Suica と社員証など）が重なって入っている場合、電波干渉によって正常に読み取れない、または `UNSUPPORTED_CARD` になることがあります。1枚ずつかざしてください。
