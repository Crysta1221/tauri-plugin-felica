# FeliCa プラグイン再設計仕様書 (PLAN.md)

本ドキュメントは `tauri-plugin-felica` の正本計画書である。
本リポジトリで作業する全てのエージェントは、本仕様書および [AGENTS.md](AGENTS.md) に従って実装を行うこと。

推測でバイトレイアウトやコマンド形を書き換えないこと。優先順位は §1.3。

---

## 1. 概要と基本方針

### 1.1 背景と目的

PaSoRi（RC-S380 / RC-S300 等の USB リーダー）を介して、Tauri v2 アプリケーションから FeliCa カードを安全・高速・高精度に読み取る。
従来のプロトタイプにおける「解析ロジックの散乱」「読取漏れ」「不要な探索による遅延・セキュリティリスク」を根本解決し、**高レベル自動認識（`scan`）** と **低レベル自由アクセス（`read` / `poll` → `FelicaCard`）** の二層で再構成する。

### 1.2 設計方針

1. **推測・勝手読みの排除**
   - 未知カードに対する Search Service Code の無断探索（旧 `dump_unknown_system`）は廃止する。
   - 認識できないカードは `UNSUPPORTED_CARD` とする。生ブロックを「未知製品」として返さない。
2. **二層 API**
   - High-Level: `reader.scan()` → パース済み `ScanResult`。
   - Low-Level: `reader.read(services)`（ワンショット）および `reader.poll()` → `FelicaCard`（対話）。
3. **責務分離**
   - Rust (`src/rust/cards/`): Request Service、Read Without Encryption、バイト列の数値解釈。
   - TS (`src/typescript/labels/` + `enrich.ts`): 駅名・事業者名・処理種別名。巨大 CSV を Rust に埋め込まない。
   - TS API: 薄い型付きラッパー。UI は持たない。
4. **システムごとに IDm が異なる**（UM 2.3.4 / 3.2.4）。読み取り対象のシステムコードを常に持ち、切替は Polling またはそのシステムの IDm をコマンドに載せる。

### 1.3 資料の優先順位

| 優先 | 資料 | 使う範囲 |
| --- | --- | --- |
| 1 | FeliCa カード ユーザーズマニュアル抜粋版 2.0（2012） | プロトコル、IDm/PMm、サービス属性、Purse LE、無鍵コマンド形。Lite・発行・暗号 Read/Write の詳細は対象外または非開示 |
| 2 | [suica-viewer `card_data.rs`](https://github.com/soltia48/suica-viewer/blob/main/src/card_data.rs) | 交通系無鍵 5 サービスのバイト解釈。jennychan より新しい |
| 3 | [felica-rs](https://github.com/soltia48/felica-rs) | フレーム生成。Search Service Code（UM 抜粋はパケット非開示） |
| 4 | jennychan（2008–2010） | 「どの無鍵サービスがあるか」、WAON/Edy 履歴の穴埋め。Byte 8「カード種別」等の古い推測は採用しない |
| 5 | 現行ソースの欠陥一覧（§2.7） | 直すべき既存バグ |

jennychan はシステムコードを `0300` のように LE で書くことがある。プラグイン内部の論理値は `0x0003`（BE 解釈）で持つ。

### 1.4 やらないこと

| 禁止 | 理由 |
| --- | --- |
| Read With Encryption / Authentication1/2 / Read / Write（暗号） | 無鍵領域のみ。氏名・定期・発行者は原理的に取れない |
| Write Without Encryption を含む書き込み全般 | 破壊的操作 |
| Lite-S の MAC 検証 | 本プラグインの範囲外。Lite 判定は Polling のみ |
| iD の推定 | 社員証等へ誤適合する |
| `scan` 中の Search Service Code | 未知サービスの無断読み |
| WAON `0x67CF`（会員情報）を `scan` で読む | PII。番号は `0x684F` で足りる |
| システムコード `0xFFFF` の捏造 | Lite で `dump_unknown_system` が発火する現行バグの根 |
| TS での hex 再パース | `rawBlockHex` は表示専用 |

---

## 2. 確定している技術的事実

### 2.1 プロトコル（UM 抜粋版）

既定のバイトオーダーはビッグエンディアン。例外は備考に《リトルエンディアン》とあるもの（サービスコード、エリアコード、Purse データ、3 バイトブロック番号）。

| コマンド | CC/RC | 要点 |
| --- | --- | --- |
| Polling | 00h / 01h | RC `00`=追加なし / `01`=システムコード / `02`=通信性能。応答は IDm+PMm[+2]。**`FFFFh` は常にシステム 0 が応答する**（4.4.2）。ワイルドカード 1 発では Lite / QUICPay / 地域交通は取れない |
| Request Service | 02h / 03h | ノード 1–32。サービスコード LE。欠落時の鍵バージョン `FFFFh`。システム鍵を取るときはノード `FFFFh` |
| Request Response | 04h / 05h | カード存在と Mode。本プラグインの scan では必須にしない |
| Read Without Encryption | 06h / 07h | **Mode0 のみ**。サービス 1–16。サービスコード LE。ブロックリストは 2 バイト（番号 &lt; 256）または 3 バイト（番号は LE）。混在可。先に Request Service で存在確認すること。**SF1 が 00h のときだけ**ブロック数とブロックデータが付く |
| Request System Code | 0Ch / 0Dh | システム 0 から列挙（コードは BE）。失敗しても `0xFFFF` を入れない。モバイル FeliCa は全件返さないことがある |
| Search Service Code | 0Ah / 0Bh | UM 抜粋はパケット非開示。低レベルのみ felica-rs 準拠。`scan` では使わない |
| Write Without Encryption / 認証系 | — | **実装しない** |

**IDm:** 複数システムがある場合、システムごとに異なる。製造者コード先頭 1 バイトの上位 4 ビットがシステム番号。

**システム切替（UM 3.2.4）:** (1) 対象システムコードで Polling する、(2) 対象システムの IDm をコマンドに載せる。既知 IDm があれば再 Polling しなくてよい。切替後は Mode0。

**PMm:** D8–D9 が IC コード、D10–D15 が最大応答時間。Read Without Encryption は D13。公式式:

```
T = 256 × 16 / fc  ≈ 0.302 ms
最大応答時間 (ms) = T × [(B+1)×n + (A+1)] × 4^E
```

各バイトは E（2bit）+ B（3bit）+ A（3bit）。本プラグインの同時読み出しは **4 ブロック固定（安全側）** とし、PMm は保持するがチャンクサイズは変えない。

**サービス属性（下位 6 ビット、UM 表 3-2）:**

| 値 | 意味 |
| --- | --- |
| `001011b` = `0x0B` | Random RO 無鍵 |
| `001111b` = `0x0F` | Cyclic RO 無鍵 |
| `010111b` = `0x17` | Purse RO 無鍵 |

無鍵 RO は奇数。

**Purse 読み出し時ブロック（UM 図 3-15）:** パースデータ（残高）とキャッシュバックは LE。Edy `0x1317` / nanaco `0x5597` / WAON `0x6817` の残高は **先頭 4 バイト u32 LE**。ヘルパーは `purse_balance(bytes) -> u32` を 1 つだけ置く。

**ステータスフラグ:** SF1 `00h` = 成功。リスト付きコマンドのエラーでは SF1 が位置、SF2 が詳細（`A8h` = ブロック番号が範囲外、など）。失敗は `BlockReadError` に SF1/SF2 を残し、`zip` でブロックを割り当てない。

### 2.2 交通系サイバネ

本節のバイトレイアウトは [suica-viewer `src/card_data.rs`](https://github.com/soltia48/suica-viewer/blob/main/src/card_data.rs) が正本。jennychan の「008B Byte 8 = カード種別・地域」は**採用しない**。

#### 2.2.1 無鍵で読めるのは 5 サービスだけ

| サービス | 内容 | 鍵 | 本プラグイン |
| --- | --- | --- | --- |
| `0x004A` | 発行情報（氏名・電話番号・生年月日・デポジット・発行事業者・発行駅・有効期限） | 必要 | **対象外** |
| `0x0816` | その他 | 必要 | **対象外** |
| `0x08CA` | 最終チャージ情報 | 必要 | **対象外** |
| `0x100A` | 拡張情報（定期券番号・定期発売額・オートチャージ） | 必要 | **対象外** |
| `0x104A` | 定期情報 | 必要 | **対象外** |
| `0x008B` | 属性情報（残高・通番・設定フラグ） | 不要 | 対象 / 1 ブロック |
| `0x090F` | 取引履歴 | 不要 | 対象 / 20 ブロック（存在時） |
| `0x10CB` | SF 改札入場情報 | 不要 | 対象 / 2 ブロック |
| `0x108F` | 改札入出場情報 | 不要 | 対象 / 3 ブロック（`detail`） |
| `0x184B` | 料金発券・改札情報 | 不要 | 対象 / 2 ブロック（`detail` かつ存在時） |

⇒ **氏名・定期券・カード種別・発行事業者・有効期限は無鍵では取れない。**

**EX-IC:** `0x008B` はあるが `0x090F` が無い個体がある。transit として成立させ、`dump_plan` は found に無い `0x090F` を要求しない。`histories` は空配列、`balance` は `0x008B` から。parse は失敗させない。

#### 2.2.2 `0x008B` 属性情報（1 ブロック）

| バイト | 内容 | 読み方 |
| --- | --- | --- |
| 8 | 設定フラグ | bit2 = タッチでGo、bit4 = 音声案内、bit5 = SF外定期 |
| 11–12 | SF 残高 | **u16 LE** → DTO では u32 |
| 14–15 | 取引通番 | **u16 BE** → DTO では u32 |

20,000 円ヒューリスティックは廃止する。

#### 2.2.3 `0x090F` 取引履歴（20 ブロック）

| バイト | 内容 | 読み方 |
| --- | --- | --- |
| 0 | 機器種別 | u8。**`0x00` なら未記録。そこで `break`（filter で詰めない）** |
| 1 | 取引種別 | **`block[1] & 0x7F`**。生値も `processTypeRaw` として残す |
| 2 | 支払種別 | u8 |
| 3 | 改札機指示種別 | u8 |
| 4–5 | 日付 | u16 BE（7bit 年 + 4bit 月 + 5bit 日、2000 年起算） |
| 6–7 | 鉄道: 入場（線区, 駅順） / バス: 事業者 u16 BE / 物販: 時刻 u16 BE | 取引種別で分岐 |
| 8–9 | 鉄道: 出場（線区, 駅順） / バス: 停留所 u16 BE | 取引種別で分岐 |
| 10–11 | 取引後残高 | **u16 LE** |
| 12–14 | 取引通番 | **u24 BE** |
| 15 | 地区コード 2 つ | 入場 = `(b15>>6)&3`、出場 = `(b15>>4)&3` |

利用金額フィールドは無い。`amount` は隣接エントリの残高差分（新しい − 古い）。最古は `null`。

物販時刻は 5bit 時 + 6bit 分 + 5bit 秒/2。**秒の 2 秒分解能を出力する**（suica-viewer は秒なしだがフィールドは存在する）。

#### 2.2.4 `0x10CB` SF 改札入場情報（2 ブロック）

- block 0: `[0]` 入場線区、`[1]` 入場駅順
- block 1: 中間改札記録（日付・時刻・入出場駅）。落としてはならない
- `hasRecord`: 32 バイトのいずれかが非ゼロ
- `processType == 0x14` による推測は禁止（0x14 は入場時オートチャージ）

#### 2.2.5 `0x108F`（3 ブロック、`detail`）

| バイト | 内容 |
| --- | --- |
| 0 | 入出場種別 |
| 1 | 中間改札機指示種別 |
| 2–3 | 駅（線区, 駅順） |
| 4–5 | 機器 ID |
| 6–7 | 日付 u16 BE |
| 8–9 | 時刻 BCD `HH:MM` |
| 10–11 | 金額 u16 LE |
| 12–13 | 定期券運賃 u16 LE |
| 14–15 | 定期区間駅（線区, 駅順） |

全ゼロはスキップ。

#### 2.2.6 `0x184B`（2 ブロック、`detail` かつ存在時）

| バイト | 内容 |
| --- | --- |
| 0–1 | 出発駅 |
| 2–3 | 到着駅 |
| 4–5 | 有効期限 u16 BE |
| 6–7 | 発行時刻 u16 BE |
| 8 | 発行種別 |
| 9 | 金額（値 × 10 円） |
| 10–11 | 機器 ID |
| 12–13 | 改札駅 |

#### 2.2.7 地区コードと駅名

- 地区は 0–3 の 2 ビット。
- `0x090F` は入場・出場の地区で一意検索。
- `0x10CB` / `0x108F` / `0x184B` は地区なし → 0–3 を総当たりし **candidates を全部残す**。先頭一致フォールバック禁止。`label` は 1 件ならその駅名、複数なら `" or "` 連結、0 件なら線区/駅順の `0x` 表示。生の `line` / `station` は常に残す。

#### 2.2.8 検知

`systemCode == 0x0003` では SAPICA `0x865E` / PASPY `0x8592` / IruCa `0x80DE` / せたまる `0x802B` が落ちる。jennychan も同一サービスセットを記録している。

**判定は Request Service で `0x008B` または `0x090F` が存在すること。** 探索対象システムは発見済み全システム（`Detection::Service { systems: None, ... }`）。

### 2.3 WAON

システムは多くの個体で `0xFE00`。ブランド SC `0x8B61` / `0x852B` も候補に含める。

| サービス | 内容 | scan |
| --- | --- | --- |
| `0x6817` | 残高 Purse 4-byte LE | 常時 1 ブロック |
| `0x680B` | 履歴 9 ブロック | 常時。**履歴ペアは blockIndex 0–5 の 3 組だけ**。6–8 は履歴にしない（jennychan） |
| `0x684B` | ポイント 3 ブロック | 存在時。endian はフィクスチャ待ち。欠落可 |
| `0x684F` | WAON 番号 BCD 2 ブロック | 存在時 |
| `0x67CF` | 会員情報 | **scan しない** |
| `0x67CB` 他 | 発行・不明 | scan しない。低レベル `read` で明示指定すれば読める |

`0x680B` 奇数ブロックのビットパック（jennychan）: 年? 5bit（オフセット 2005）+ 月 4bit + 日 5bit + 時 5bit + 分 6bit + 残額 18bit + 利用金額 18bit + チャージ 17bit。偶数 = 端末 / 通番。ペアは `blockIndex` の偶奇。奇数欠落のペアは捨て、ずらさない。

### 2.4 Edy / nanaco / QUICPay / Lite / iD

**Edy**（多くの個体は `0xFE00`、候補に `0x811D`）:

- `0x1317` 残高 Purse u32 LE
- `0x170F` 履歴 6 ブロック。日時は 2000-01-01 起算（15bit 日 + 17bit 秒）。金額・残高は u32。符号付き `<<` 禁止
- `0x110B` 属性 2 ブロック。Edy ID は BCD（jennychan）。欠落可

**nanaco**（`0xFE00` および `0x04C7`）:

- `0x5597` 残高 Purse u32 LE（BE 禁止）
- `0x564F` 履歴 5 ブロック。年ビット幅はフィクスチャで確定してから定数化
- `0x560B` ポイント 2 ブロック（存在時）
- `0x558B` 会員番号 2 ブロック（存在時）

**QUICPay:** `Polling(0x04C1)` のみ。公開無鍵サービスなし。identity だけ返す。

**Lite / Lite-S:** UM はこのシリーズを対象としない。判定は `Polling(0x88B4)` のみ（Request Service / Request System Code の可否をこの PDF から断定しない）。読むのは `0x000B` の S_PAD0–13（0x00–0x0D）。NDEF 解釈は scan 必須ではない。

**iD:** 無鍵の識別サービスが無い。推定禁止。`UNSUPPORTED_CARD`。

### 2.5 現行実装の既知欠陥（本 PLAN で排除する）

暗号化領域は対象外としたうえで、無鍵の読み間違いを列挙する。Step 9 に進む前に全項目の解消を確認すること。

凡例: **確定バグ** / **設計欠陥** / **要実機確認**。

#### A. 交通系 — `src/typescript/parse/transit.ts` / `labels/cybernetics.ts`

| # | 内容 | 判定 |
| --- | --- | --- |
| A-1 | 通番を `[12]<<8\|[13]` で取っている。正は u24 BE 12–14 | 確定バグ |
| A-2 | `processType` の `& 0x7F` が経路で食い違う。`0x94` を取りこぼす | 確定バグ |
| A-3 | 支払種別・改札機指示を DTO に持たず `rawBlockHex` を再デコード | 設計欠陥 |
| A-4 | Byte 15 を単一 region として保持。入場/出場の 2 ビットに分解すること | 確定バグ |
| A-5 | 全ゼロ filter で履歴を詰める。正は `[0]==0` で break | 確定バグ |
| A-6 | 駅名が無いと線区・駅順ごと破棄する経路がある | 設計欠陥 |
| A-7 | 地区なし駅を先頭 1 件に決め打ち。candidates 全件 | 設計欠陥 |
| A-8 | 残高を 20,000 円ヒューリスティック。`0x008B` を読むこと | 確定バグ |
| A-9 | 入場を `processType===0x14` で推測。`0x10CB` を読むこと | 確定バグ |
| A-10 | 処理・端末のマジックナンバー。定数へ | 設計欠陥 |
| A-11 | 物販秒の 2 秒分解能を出力仕様とする | 要実機確認 |
| A-12 | `systemCode==0x0003` 以外を拒否。サービス検知へ | 確定バグ |

#### B. 電子マネー — `src/typescript/parse/emoney.ts`

| # | 内容 | 判定 |
| --- | --- | --- |
| B-1 | nanaco 残高 BE | 確定バグ |
| B-2 | WAON 残高 2 バイト LE | 確定バグ |
| B-3 | JS の符号付き `<< 24` | 確定バグ |
| B-4 | WAON 履歴を配列添字でペア。`blockIndex` へ。かつ 0–5 のみ | 確定バグ |
| B-5 | 偶数ブロック（端末・通番）を捨てている | 設計欠陥 |
| B-6 | 利用金額・チャージが無い | 設計欠陥 |
| B-7 | iD 推定 | 確定バグ |
| B-8 | 先勝ち 1 ブランド。`ScanResult` で全製品 | 確定バグ |
| B-9 | 全ゼロブロックでブランド判定。Request Service へ | 設計欠陥 |
| B-10 | nanaco 年 11bit が過大の可能性 | 要実機確認 |

#### C. プロトコル — `protocol/response.rs` / `command.rs` / `dump.rs`

| # | 内容 | 判定 |
| --- | --- | --- |
| C-1 | `felica_pdu` が応答コード一致位置を総当たり。長さ+IDm で検証すること | 設計欠陥 |
| C-2 | Read 応答の IDm 未検証。全パーサで照合 | 設計欠陥 |
| C-3 | `parse_system_codes` だけ `payload_offset` 不使用 | 確定バグ |
| C-4 | `zip` で不足ブロックを先頭に割り当て。件数不一致はエラー記録 | 設計欠陥 |
| C-5 | 失敗を `continue`。`BlockReadError` へ | 設計欠陥 |
| C-6 | 全ブロックがサービス index 0 の 2 バイト要素。index と 3 バイト要素に対応 | 設計欠陥 |
| C-7 | PMm を捨てている。保持する。チャンク 4 は据え置き | 設計欠陥 |
| C-8 | Polling RC が常に `00`。ワイルドカードは `01` | 設計欠陥 |

#### D. スキャン統括 — `desktop.rs` / `dump.rs`

| # | 内容 | 判定 |
| --- | --- | --- |
| D-1 | 低レベル read がワイルドカード IDm のまま。`ServiceRead.system_code` 必須 | 確定バグ |
| D-2 | RSC 失敗時に `0xFFFF` を捏造 → 未知探索が発火。空にする | 確定バグ |
| D-3 | ダンプ中のカード入れ替わり未検出。完了後に再 Polling | 設計欠陥 |
| D-4 | `select_system` 失敗でワイルドカード IDm 継続。スキップすること | 確定バグ |
| D-5 | 共通領域 `0xFE00` を常に優先しブランド SC を試さない | 設計欠陥 |
| D-6 | 存在確認なしで 6 サービスを読む | 設計欠陥 |
| D-7 | 交通系が `0x090F` のみ。`0x008B` / `0x10CB` を追加 | 確定バグ |
| D-8 | `timeoutMs` を最大 280ms 超過。締切と残り時間で切る | 設計欠陥 |

#### E. 型・エラー

| # | 内容 | 判定 |
| --- | --- | --- |
| E-1 | 判別子 `kind` → `type: FelicaType` | 設計欠陥 |
| E-2 | `maxServices` / `maxBlocksPerService` を公開から削除 | 設計欠陥 |
| E-3 | `BlockData.systemCode` 必須。`blockIndex` は u16 | 設計欠陥 |
| E-4 | `GateStatus.isEntry` が常に true。`0x10CB.hasRecord` へ | 設計欠陥 |
| E-5 | `serialize_struct(..., 2)` 直書き。**4 フィールド**（code / message / systemCodes / detected）を derive する。2→3 では足りない | 設計欠陥 |
| E-6 | `CARD_TYPE_MISMATCH` と TS の `INTERNAL_ERROR` を追加 | 設計欠陥 |

#### 2.5.1 横断ルール

1. 生データは捨てない（線区・駅順・地区・hex）。
2. 数値解釈は 1 箇所。生値 `raw` とマスク済み `code` を両方持ち、判定は `code`。
3. hex 再パース禁止。
4. 配列添字を意味に使わない。参照キーは `systemCode + serviceCode + blockIndex`。
5. 全レスポンスで IDm 検証。ダンプ後に再 Polling。
6. 失敗は理由付き。存在しないと読めなかったを区別。
7. 値を捏造しない。

---

## 3. スキャン手順（唯一の正本）

実装は `src/rust/cards/scan.rs`（手順）と `src/rust/cards/rf.rs`（Polling / Request Service / Read）。`desktop.rs` はセッション・締切・排他だけを持つ。`protocol/dump.rs` は削除する。

### 3.1 ステップ

| # | 動作 | 失敗時 |
| --- | --- | --- |
| 0 | `deadline = now + timeoutMs`（0/省略は無限）。以後の transceive は `min(固定値, 残り)` | 残り 0 で `SCAN_TIMEOUT`。**カード検出後のダンプには timeoutMs を適用しない** |
| 1 | `Polling(SC=0xFFFF, RC=0x01, slot=0x00)`。IDm / PMm / 任意の 2 バイト SC | 無応答は 200ms 待って再試行。キャンセルは `SCAN_CANCELLED`。これは **システム 0 だけ** |
| 2 | Request System Code。結果を `systemCodes` | 失敗なら捏造しない。ステップ 1 の SC があればそれだけ、無ければ空 |
| 3 | 追加 Polling: `0x88B4`（Lite）、`0x04C1`（QUICPay）。`targets` で絞っているならその対象だけ | 無応答は「その製品は無い」。エラーにしない |
| 4 | 候補 = (`systemCodes` ∪ プロファイル提示の FE00/ブランド SC) のユニーク。各システムで Polling → その IDm で Request Service | Polling 失敗のシステムはスキップ。IDm 不一致は `PROTOCOL_ERROR` |
| 5 | `keyVersion != 0xFFFF` を found にする。`Detection` で照合 | 下記 §3.3 |
| 6 | マッチしたプロファイルの `dump_plan(found, detail)` を結合。同一 `(system, service)` は一度だけ | found に無いサービスは計画に入れない |
| 7 | 対象システムの IDm で Read Without Encryption。チャンク 4。切替は再 Polling でも既知 IDm でもよい | SF1≠00h ならブロックデータ無し。`BlockReadError` に記録。製品全体は落とさない |
| 8 | 再 `Polling(0xFFFF, RC=0x01)` で IDm 一致 | 不一致は `PROTOCOL_ERROR`（入れ替わり） |
| 9 | プロファイルごとに `parse`。`Err` はその製品だけ欠落 | 全製品が落ちたら `UNSUPPORTED_CARD` |

往復の目安: 交通系 23 ブロックで最低 6 往復、`detail` で +2 サービス。これが `detail` 既定 false の根拠。

### 3.2 Detection

```rust
pub enum Detection {
    /// Polling に応答すれば検出（Lite, QUICPay）。
    Polling(&'static [u16]),
    /// Request Service。`systems == None` はステップ 4 の全候補。
    /// `Some(&[0xFE00, 0x811D])` のようにブランド候補を足す。
    Service {
        systems: Option<&'static [u16]>,
        any_of: &'static [u16],
    },
}

pub struct ServiceRead {
    pub system_code: u16,
    pub service_code: u16,
    pub blocks: Vec<u16>,
}

pub trait CardProfile: Send + Sync {
    fn card_type(&self) -> FelicaType;
    fn detection(&self) -> Detection;
    fn dump_plan(&self, found: &[u16], detail: bool) -> Vec<ServiceRead>;
    fn parse(&self, blocks: &[BlockData]) -> Result<ParsedProduct>;
}
```

`Detection::Service { system: u16 }`（システム 1 個固定）は禁止。地域交通と FE00 上の Edy/WAON が落ちる。

| プロファイル | Detection | dump_plan |
| --- | --- | --- |
| transit | `Service { systems: None, any_of: [0x008B, 0x090F] }` | found にあれば 008B:0 / 090F:0–19 / 10CB:0–1。detail なら 108F:0–2。184B は存在時のみ 0–1 |
| waon | `Service { systems: Some(&[0xFE00, 0x8B61, 0x852B]), any_of: [0x6817] }` | 6817:0 / 680B:0–8 / 684B:0–2（存在時） / 684F:0–1（存在時） |
| edy | `Service { systems: Some(&[0xFE00, 0x811D]), any_of: [0x1317] }` | 1317:0 / 170F:0–5 / 110B:0–1（存在時） |
| nanaco | `Service { systems: Some(&[0xFE00, 0x04C7]), any_of: [0x5597] }` | 5597:0 / 564F:0–4 / 560B:0–1（存在時） / 558B:0–1（存在時） |
| quicpay | `Polling(&[0x04C1])` | 読まない |
| lite | `Polling(&[0x88B4])` | `0x000B` の 0x00–0x0D |

`parse` の `Err` はオーケストレータが握り潰し、その製品だけ落とす。

### 3.3 `targets` と `require`

`require: true` = **targets に一致するまで待機**（対話合意・本 PLAN の JSDoc）。`false`（既定）= 即 `CARD_TYPE_MISMATCH`。

`targets` なしの `require: true` は無視する。
**既知製品が 1 つも無いときは `require` に関係なく `UNSUPPORTED_CARD`**（社員証を無限待機しない）。

| targets | require | かざしたカード | 結果 |
| --- | --- | --- | --- |
| なし | — | 既知あり | 全製品の `ScanResult` |
| なし | — | 既知なし | `UNSUPPORTED_CARD`（`systemCodes` 付き、捏造しない） |
| `[TRANSIT]` | `false` | Suica | transit のみ読む |
| `[TRANSIT]` | `false` | WAON | `CARD_TYPE_MISMATCH` `detected=['waon']` |
| `[TRANSIT]` | `true` | WAON | 無視して待機 → 一致 or `SCAN_TIMEOUT` |
| `[TRANSIT]` | どちらでも | 社員証 | 既知なしなので `UNSUPPORTED_CARD`（待機しない） |

`targets` は **Read する製品を絞る**。検知の Request Service / 追加 Polling は、待機判定のために既知プロファイルへ行う。一致しない製品のブロックは読まない。

---

## 4. TypeScript API と DTO

### 4.1 公開関数

| 呼び出し | 戻り値 | 例外 |
| --- | --- | --- |
| `FelicaReader.list()` | `Promise<ReaderInfo[]>` | 空配列は正常 |
| `FelicaReader.connect(opts?)` | `Promise<FelicaReader>` | `DEVICE_NOT_FOUND` / `DEVICE_ACCESS_DENIED` / `DEVICE_BUSY` / `UNSUPPORTED_DEVICE` |
| `reader.scan(opts?)` | `Promise<ScanResult>` | `UNSUPPORTED_CARD` / `CARD_TYPE_MISMATCH` / `SCAN_TIMEOUT` / `SCAN_CANCELLED` / `PROTOCOL_ERROR` / `DEVICE_*` |
| `reader.read(services)` | `Promise<ReadBlocksResult>` | 各要素に `systemCode` 必須。内部で 1 回 Polling。自動解放 |
| `reader.poll(systemCode?)` | `Promise<FelicaCard>` | 省略時 `0xFFFF` RC=`0x01`。要 `release()`。held / scan 中は `DEVICE_BUSY` |
| `reader.cancelScan()` | `Promise<void>` | 無セッションは `SESSION_CLOSED` |
| `reader.disconnect()` | `Promise<void>` | held も失効 |
| `card.idm` / `card.pmm` | `string` | 選択中システム |
| `card.systemCodes` | `number[]` | 未取得なら空 |
| `card.requestService(codes)` | `Promise<ServiceStatus[]>` | `exists = keyVersion !== 0xFFFF` |
| `card.searchServices({ from?, max? })` | `Promise<ServiceNode[]>` | ブロックは読まない。パケットは felica-rs 準拠 |
| `card.selectSystem(systemCode)` | `Promise<{ idm, pmm }>` | 再 Polling。`card.idm` 更新 |
| `card.read(serviceCode, blocks)` | `Promise<ReadBlocksResult>` | 現在システム。`blocks` は `number`（u16） |
| `card.release()` | `Promise<void>` | 冪等 |

`FelicaCard.parse` / `FelicaParser` / `FelicaBrand` / `maxServices` は削除する。現行の `FelicaCard` クラスは低レベルハンドルに置き換える。

### 4.2 ScanOptions

```ts
export const FelicaType = {
  TRANSIT: "transit",
  WAON: "waon",
  EDY: "edy",
  NANACO: "nanaco",
  QUICPAY: "quicpay",
  LITE: "lite",
} as const;
export type FelicaType = (typeof FelicaType)[keyof typeof FelicaType];

export interface ScanOptions {
  /** 読む製品。省略時は検知した全既知プロファイル */
  targets?: FelicaType[];
  /** true なら targets に一致するまで待機。既定 false = 即 CARD_TYPE_MISMATCH */
  require?: boolean;
  /** 交通系 0x108F / 0x184B を追加。既定 false */
  detail?: boolean;
  /** カード検出までのミリ秒。省略または 0 で無限。検出後のダンプには使わない */
  timeoutMs?: number;
  signal?: AbortSignal;
}
```

### 4.3 共通型

```ts
export interface BlockData {
  systemCode: number;
  serviceCode: number;
  blockIndex: number; // u16
  dataHex: string;    // 32 文字大文字
}

export interface BlockReadError {
  systemCode: number;
  serviceCode: number;
  blockIndex: number;
  reason: "STATUS_FLAG" | "COMMUNICATION" | "PROTOCOL" | "WRONG_IDM";
  statusFlag1?: number;
  statusFlag2?: number;
}

export interface ReadBlocksResult {
  blocks: BlockData[];
  errors: BlockReadError[];
}

export interface LabeledCode {
  code: number;
  label: string; // 未知なら "0xNN"
}

export interface StationCandidate {
  region: number;
  companyName: string;
  lineName: string;
  stationName: string;
}

export interface StationRef {
  line: number;
  station: number;
  region?: number;
  label: string;
  candidates?: StationCandidate[];
}

export interface ServiceStatus {
  serviceCode: number;
  keyVersion: number;
  exists: boolean;
}

export interface ServiceNode {
  index: number;
  code: number;
  kind: "area" | "service";
}

export interface ServiceRead {
  systemCode: number;
  serviceCode: number;
  blocks: number[];
}
```

### 4.4 ScanResult（TS クラス）

Rust は `products` を送らない。TS が `new ScanResult(dto)` でラップし、getter で導出する。labels はコンストラクタ内で一度だけ `enrichScanResult` する。

```ts
export class ScanResult {
  readonly idm: string;  // Wildcard Polling の IDm（システム 0）
  readonly pmm: string;
  readonly systemCodes: number[];
  readonly transit?: TransitCard;
  readonly waon?: WaonCard;
  readonly edy?: EdyCard;
  readonly nanaco?: NanacoCard;
  readonly quicpay?: QuicpayCard;
  readonly lite?: LiteCard;
  readonly blocks: BlockData[];
  readonly errors: BlockReadError[];
  get products(): FelicaType[];
}
```

成功時 `blocks` は常にある（QUICPay は空配列）。欠落ブロックに依存するフィールドは `undefined`。製品全体は落とさない。

### 4.5 製品 DTO

判別子はすべて `type`（`kind` 禁止）。各製品は自身の `systemCode` と `idm` を持つ。

```ts
export interface TransitSettings {
  raw: number;
  touchDeGo: boolean;
  voiceGuidance: boolean;
  sfOutsideCommuter: boolean;
}

export interface TransitGate {
  hasRecord: boolean;
  entry?: StationRef; // 0x10CB block0。地区なし → candidates
  intermediate?: {
    entryDate?: string;
    entryTime?: string;
    entry?: StationRef;
    exitTime?: string;
    exit?: StationRef;
    unknown1Hex?: string;
    unknown2Hex?: string;
  };
}

export interface TransitHistoryEntry {
  rawBlockHex: string;
  blockIndex: number;
  terminalType: LabeledCode;
  processTypeRaw: number;
  processType: LabeledCode; // code = raw & 0x7F
  paymentType: LabeledCode;
  gateInstructionType: LabeledCode;
  date: string;
  time?: string; // 物販は秒あり（2 秒分解能）
  entry?: StationRef;
  exit?: StationRef;
  busCompanyCode?: number;
  busStopCode?: number;
  busCompanyName?: string;
  balance: number;
  amount?: number | null; // 最古は null
  seqNumber: number; // u24
  entryRegion: number;
  exitRegion: number;
}

export interface TransitGateRecord { /* 0x108F。生コード + StationRef + rawBlockHex */ }
export interface PaidTicket { /* 0x184B */ }

export interface TransitCard {
  type: "transit";
  systemCode: number;
  idm: string;
  balance?: number;
  seqNumber?: number;
  settings?: TransitSettings;
  gate: TransitGate;
  histories: TransitHistoryEntry[];
  gateRecords?: TransitGateRecord[];
  paidTickets?: PaidTicket[];
}

export interface WaonHistoryEntry {
  raw: string; // 2 ブロック連結 hex
  blockIndex: number; // 偶数側
  terminalId?: string;
  seqNumber?: number;
  typeCode?: number;
  dateTime?: { date: string; time: string };
  amount?: number;
  chargeAmount?: number;
  balance?: number;
}

export interface WaonCard {
  type: "waon";
  systemCode: number;
  idm: string;
  balance: number;
  points?: number;
  waonNumber?: string;
  histories: WaonHistoryEntry[];
}

export interface EdyCard {
  type: "edy";
  systemCode: number;
  idm: string;
  balance: number;
  edyNumber?: string;
  histories: Array<{
    rawBlockHex: string;
    typeCode: number;
    amount?: number;
    balance?: number;
    date?: string;
    time?: string;
  }>;
}

export interface NanacoCard {
  type: "nanaco";
  systemCode: number;
  idm: string;
  balance: number;
  points?: number;
  nanacoNumber?: string;
  histories: Array<{
    rawBlockHex: string;
    typeCode?: number;
    amount?: number;
    balance?: number;
    date?: string;
    time?: string;
  }>;
}

export interface QuicpayCard {
  type: "quicpay";
  systemCode: number; // 0x04C1
  idm: string;
}

export interface LiteCard {
  type: "lite";
  systemCode: number; // 0x88B4
  idm: string;
  sPad: Record<number, string>; // 読めたパッドのみ
}
```

### 4.6 エラー

Rust の `FelicaErrorCode` と TS は同一集合。

| code | 条件 | payload |
| --- | --- | --- |
| `UNSUPPORTED_CARD` | 既知プロファイルなし | `systemCodes?`（空可、捏造しない） |
| `CARD_TYPE_MISMATCH` | targets 不一致かつ `require: false` | `systemCodes?`, `detected: FelicaType[]` |
| `SCAN_TIMEOUT` | 検出前に締切 | なし |
| `SCAN_CANCELLED` | AbortSignal / `cancelScan` | なし |
| `SESSION_CLOSED` | `release` 済み、またはカード喪失後 | なし |
| `DEVICE_NOT_FOUND` | リーダー無し | なし |
| `DEVICE_ACCESS_DENIED` | USB が開けない | なし |
| `UNSUPPORTED_DEVICE` | Sony だが非対応チップ | なし |
| `DEVICE_DISCONNECTED` | 抜線 | なし |
| `DEVICE_BUSY` | scan / poll / held の競合 | なし |
| `COMMUNICATION_ERROR` | RF / 途中離脱 | なし |
| `PROTOCOL_ERROR` | フレーム不正、IDm 不一致、ブロック数不一致 | なし |
| `INTERNAL_ERROR` | 上記以外。TS にも追加 | なし |

```ts
class FelicaError extends Error {
  readonly code: FelicaErrorCode;
  readonly systemCodes?: number[];
  readonly detected?: FelicaType[];
}
```

- Serialize はフィールド数直書きをやめ、`#[derive(Serialize)] struct FelicaErrorBody { code, message, systemCodes, detected }` にする。
- `wrapError` は未知 `code` を `INTERNAL_ERROR` に落とす。

タイムアウト: スキャン開始時に締切を一度だけ決める。カード検出後は `timeoutMs` を適用しない。

---

## 5. Tauri コマンドとセッション

`build.rs` / `lib.rs` `generate_handler!` / `commands/mod.rs` / `permissions/default.toml` を同期する。**11 コマンド。**

| コマンド | Request | Response |
| --- | --- | --- |
| `list_readers` | なし | `ReaderInfoDto[]` |
| `connect_reader` | `{ id?, preference? }` | `{ sessionId, reader }` |
| `disconnect_reader` | `{ sessionId }` | null |
| `cancel_scan` | `{ sessionId }` | null |
| `scan_card` | `{ sessionId, timeoutMs?, targets?: string[], require?: bool, detail?: bool }` | 製品 DTO + `idm`/`pmm`/`systemCodes`/`blocks`/`errors`（`products` なし） |
| `read_blocks` | `{ sessionId, timeoutMs?, services: ServiceReadDto[] }` | `{ blocks, errors }` |
| `poll_card` | `{ sessionId, timeoutMs?, systemCode?: u16 }` | `{ idm, pmm, systemCodes }` |
| `release_card` | `{ sessionId }` | null |
| `request_service` | `{ sessionId, serviceCodes: u16[] }` | `{ results: { serviceCode, keyVersion }[] }` |
| `search_services` | `{ sessionId, startIndex?: u16, maxNodes?: u32 }` | `{ nodes: { index, code, kind }[] }` |
| `select_system` | `{ sessionId, systemCode: u16 }` | `{ idm, pmm }` |

`DeviceSession` に `held: Option<HeldCard>` を追加する。`HeldCard = { idm, pmm, current_system, system_codes }`。

`scan_card` / ワンショット `read_blocks` / `poll_card` は `scanning` フラグで排他。held 中の scan は `DEVICE_BUSY`。`disconnect_reader` と `release_card` で held をクリア。

serde は `camelCase`。ブロック番号は `u16`。

---

## 6. ファイル構造

`hardware/`（port100 / port400 / rcs320 / rcs956）は触らない。`transceive` のタイムアウト引数をオーケストレータが渡すだけ。

### 6.1 Rust

| パス | 責務 |
| --- | --- |
| `src/rust/lib.rs` | `mod cards`。handler を 11 コマンドに |
| `src/rust/build.rs` | コマンド名配列 |
| `src/rust/error.rs` | `CardTypeMismatch`。Serialize Body。`INTERNAL_ERROR` 維持 |
| `src/rust/desktop.rs` | HeldCard、締切、排他。ダンプロジックは持たない |
| `src/rust/protocol/command.rs` | `build_request_service`。Read を `(service_index, block_u16)` 対応に。Polling の RC は引数（呼び出し側が `0x01`） |
| `src/rust/protocol/response.rs` | 長さ+応答コード+IDm。`parse_polling -> (idm, pmm, Option<u16>)`。`parse_request_service`。system codes を `payload_offset` 経由。全パーサが `expected_idm` |
| `src/rust/protocol/dump.rs` | **削除** |
| `src/rust/cards/mod.rs` | 再エクスポート |
| `src/rust/cards/profile.rs` | Detection, ServiceRead, CardProfile, レジストリ |
| `src/rust/cards/scan.rs` | §3 のステップ 0–9 |
| `src/rust/cards/rf.rs` | Polling / Request Service / Read Without Encryption / Search / Select |
| `src/rust/cards/decode.rs` | `le16` / `be16` / `be24` / `purse_balance` / `bcd` |
| `src/rust/cards/transit.rs` | 無鍵 5 サービスのみ |
| `src/rust/cards/waon.rs` | 680B は 0–5 のみペア |
| `src/rust/cards/edy.rs` | |
| `src/rust/cards/nanaco.rs` | |
| `src/rust/cards/quicpay.rs` | identity |
| `src/rust/cards/lite.rs` | S_PAD |
| `src/rust/cards/*` の `#[cfg(test)]` | hex フィクスチャ（パーサと同ファイル） |
| `src/rust/models/scan.rs` | Scan の REQ/RES。`maxServices` 削除 |
| `src/rust/models/blocks.rs` | `BlockDataDto` + `system_code` + `block_index: u16` + `BlockReadErrorDto` |
| `src/rust/models/card.rs` | 製品 DTO |
| `src/rust/models/low_level.rs` | poll / read / request_service / search / select_system |
| `src/rust/commands/<name>.rs` | 既存 5 + 新規 6（read_blocks, poll_card, release_card, request_service, search_services, select_system） |

### 6.2 TypeScript

| パス | 責務 |
| --- | --- |
| `src/typescript/index.ts` | 再エクスポート。`FelicaParser` / `FelicaBrand` 削除 |
| `src/typescript/reader.ts` | `FelicaReader` |
| `src/typescript/card.ts` | 低レベル `FelicaCard` ハンドル |
| `src/typescript/scan-result.ts` | `ScanResult` クラス |
| `src/typescript/types.ts` | `FelicaType` と全 DTO |
| `src/typescript/error.ts` | 13 コード + payload + `wrapError` 検証 |
| `src/typescript/enrich.ts` | labels 適用。hex 再パース禁止 |
| `src/typescript/parse/` | **削除** |
| `src/typescript/labels/*` | 維持。駅 lookup を複数候補 API に |
| `scripts/generate-labels.ts` | **新規。** `assets/*.csv` → maps。`examples/scripts` の `ekicode.csv` は使わない |

現行の手書き maps と CSV の差分を確認してから生成物に切り替える。空の駅辞書で出荷しない。

### 6.3 権限・ドキュメント

| パス | 変更 |
| --- | --- |
| `permissions/default.toml` | `allow-read-blocks` 等を全コマンド分追加 |
| `AGENTS.md` | 「`FelicaCard.parse` はローカル」を削除。`cards/` と低レベルコマンドを追記 |
| `examples/AGENTS.md` | `scan()` が `ScanResult`。parse 廃止、`/lab` |
| `PRODUCT.md` | 複合カードと Lite。捏造禁止は維持 |

---

## 7. labels

- 正本 CSV: `assets/station_codes.csv`, `assets/bus_company_codes.csv`。
- 鉄道の事業者名は駅コードから引ける。`entryStationName` / `exitStationName` / `companyName` / `busCompanyName` / 処理・端末ラベルを enrich で付ける。
- 静的辞書は `tauri-plugin-felica-api/labels` としてスキャン外でも使える。
- `SYSTEM_CODE_LABELS` に `0x88B4`（Lite）、地域交通 SC（`0x802B` 等）を足す。

---

## 8. examples

現行は `session.scan()` → `FelicaCard` → `card.parse(FelicaParser.Transit)` とブランド総当たり。新 API では:

```ts
const result = await reader.scan({ signal, detail, targets });
```

クライアント側 parse は禁止。残高や駅名を仮データで埋めない（PRODUCT.md）。

### 8.1 削除する呼び出し

| 現行 | 置換 |
| --- | --- |
| `FelicaCard.parse` / `FelicaParser` / `FelicaBrand` | `result.transit` 等 |
| `FelicaBrand.Id` | UI からも削除 |
| `FelicaSystemCode.Transit` / `codes.includes(0x0003)` | `result.transit` の有無 |
| 履歴配列添字の `calculateBalanceDelta` | `entry.amount`（最古は ー） |

### 8.2 `reader-scan-page.tsx`

| 箇所 | 変更 |
| --- | --- |
| ヘッダー | 「詳細読取」トグル（`detail`）。targets マルチセレクト（交通 / WAON / Edy / nanaco / QUICPay / Lite）。既定は全選択 → `targets` 未指定 |
| 残高 | ブランド別に並べる。単一の「最初の残高」は廃止 |
| 識別 | `result.products`。iD 削除。Lite / QUICPay 追加 |
| IDm | トップレベルに加え、製品があれば `systemCode` + `idm` |
| 改札 | `gate.hasRecord` なら入場中 + `StationRef.label`。false なら「入場していません」。processType 推測は使わない |
| 設定 | `touchDeGo` / `voiceGuidance` / `sfOutsideCommuter` を Badge。欠落時は出さない |
| 交通履歴 | 日時 / 処理 / 端末 / 支払 / 区間 / 金額 / 残高。入場・出場 region を別表示。raw は折りたたみ |
| WAON 履歴 | 日時 / 種別 / 利用 / チャージ / 残高 / 通番 / 端末 |
| Lite | 右ペインに S_PAD テーブル |
| QUICPay | 「公開ブロックなし」Empty |
| 生データ | `systemCode + serviceCode + blockIndex`。`errors` は別テーブル |
| エラー | `CARD_TYPE_MISMATCH` は `detected` を日本語化。`UNSUPPORTED_CARD` は `systemCodes` を `0x` 表示。`INTERNAL_ERROR` を追加 |

### 8.3 `use-reader-scan.ts`

- `ParsedScan` / `parseCard()` 廃止。`state.scan: ScanResult | null`。
- `startScan`: `session.scan({ signal, detail, targets })`。
- `errorMessage()` に `CARD_TYPE_MISMATCH` / `INTERNAL_ERROR`。
- `FelicaSystemCode` / `FelicaBrand` import 削除。

### 8.4 低レベル確認 `/lab`

PRODUCT.md はゲートシミュレータを禁止する。別ルート `/lab` を 1 枚追加し、poll / requestService / read / selectSystem / searchServices を手で確認する。shadcn のみ。Search の一括 dump ボタンは置かない。

| 操作 | API |
| --- | --- |
| カード捕捉 | `reader.poll()` |
| システム切替 | `card.selectSystem(hex)` |
| 存在確認 | `card.requestService([...])` |
| ブロック読取 | `card.read` または `reader.read([{ systemCode, serviceCode, blocks }])` |
| サービス列挙 | `card.searchServices({ max: 32 })` → コード一覧のみ |
| 解放 | `card.release()` |

`examples/src/routes/lab.tsx` を新規。TanStack Router が `routeTree.gen.ts` を再生成。`__root.tsx` に `/` と `/lab` の簡易ナビ。

### 8.5 その他

- `format.ts`: `amount` 表示。添字差分は使わない。
- `examples/AGENTS.md` / `PRODUCT.md` を §6.3 どおり更新。
- 品質ゲート: `pnpm lint`、`src-tauri` の `cargo check`。
- ブラウザでは PaSoRi 実機は動かない。空・スキャン中・エラー・成功レイアウトを確認し、実カードは人手。

---

## 9. 実装ステップ

各ステップ終了時にプラグイン側 `bun run lint` / `bun run build` / `cargo check`（警告ゼロ）。examples は Step 8 で `pnpm lint` と `src-tauri` の `cargo check`。

| Step | 内容 | 完了条件 |
| --- | --- | --- |
| 1 プロトコル | Request Service の build/parse。Polling が `Option<u16>`。全レスポンス IDm 検証。Read の 2/3 バイト要素とサービス index。system codes を `payload_offset` に統一。単体テスト | `command.rs` / `response.rs` のテスト緑 |
| 2 cards 骨格 | Detection / CardProfile / decode / scan + rf。`dump.rs` 削除。`dump_unknown_system` 廃止 | 未知システムを舐めない。FE00 優先バグを消す |
| 3 製品パーサ | transit / waon / edy / nanaco / quicpay / lite + hex フィクスチャ | A-1〜A-12 / B-1〜B-10 相当がテストで再現しない |
| 4 モデルとエラー | Scan から `maxServices` 削除。製品 DTO。`CardTypeMismatch`。Serialize Body | フィールド数直書きが消えている |
| 5 desktop + コマンド | HeldCard、締切、11 コマンド、`build.rs`、`default.toml` | permissions 再生成後に全 `allow-*` |
| 6 TS API | 型、ScanResult クラス、FelicaCard ハンドル、`parse/` 削除、複数候補 lookup、enrich | `bun run lint && bun run build`。公開 export から `FelicaParser` 消滅 |
| 7 labels スクリプト | `scripts/generate-labels.ts`。現行 maps と CSV の差分確認後に切替 | 駅辞書が空にならない |
| 8 examples | scan UI 全面、iD 削除、`/lab`、AGENTS.md / PRODUCT.md | `pnpm lint`。型エラーゼロ。仮残高なし |
| 9 品質ゲート | プラグイン + examples | 警告ゼロ。§2.5 全項目が解消 |

### 9.1 フィクスチャで確定してから定数化する項目

| 項目 | 扱い |
| --- | --- |
| WAON `0x684B` ポイント endian | 未確定。読めなければ `points` 欠落 |
| nanaco 履歴の年ビット幅 | 現行 11bit を疑う。確定後に定数化 |
| 物販秒の 2 秒分解能 | 出力する |
| Edy `0x110B` 番号位置 | 欠落許容 |
| Lite の Request Service 可否 | 判定は Polling のみ。RS 失敗は無視 |
