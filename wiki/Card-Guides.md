# カード別読み取りガイド

`ScanResult` から取得できる各カード製品（交通系IC、電子マネー、Lite等）のデータ構造と、それぞれのフィールドの意味を解説します。

---

## 1. 交通系 IC カード (`TransitCard`)

Suica、PASMO、ICOCA、TOICA、manaca、Kitaca、SUGOCA、nimoca、はやかけん、各種地域連携ICカード、EX-IC などに対応しています。

```typescript
if (result.transit) {
  const card = result.transit;
  console.log(`システムコード: 0x${card.systemCode.toString(16)}`);
  console.log(`カード残高: ¥${card.balance}`);
}
```

### 主要なプロパティ

| プロパティ | 型 | 説明 |
| :-- | :-- | :-- |
| `balance` | `number \| undefined` | カードの現在残高（円単位） |
| `seqNumber` | `number \| undefined` | カードの最終取引通番 |
| `settings` | `TransitSettings \| undefined` | カードの設定フラグ（タッチでGo、音声案内、SF外定期利用の有効/無効） |
| `gate` | `TransitGate` | 改札の入場ステータス（入場中フラグ、入場駅情報、中間改札情報） |
| `histories` | `TransitHistoryEntry[]` | 直近の利用履歴（最大 20 件、新しい順） |
| `gateRecords` | `TransitGateRecord[]` | 改札入出場履歴（`detail: true` 指定時のみ取得） |
| `paidTickets` | `PaidTicket[]` | 特急券・グリーン券・新幹線等の発券改札情報（`detail: true` 指定時のみ取得） |

### 利用履歴 (`TransitHistoryEntry`) の構造

交通系ICの取引履歴には、鉄道利用・バス利用・物販（電子マネー決済）・チャージなど多岐にわたるデータが含まれます。

```typescript
for (const entry of result.transit.histories) {
  console.log(`日付: ${entry.date}`); // "2026-03-15"
  console.log(`端末: ${entry.terminalType.label}`); // 例: "改札機", "精算機", "車載機"
  console.log(`処理: ${entry.processType.label}`); // 例: "入場・出場", "精算", "オートチャージ"
  console.log(`支払: ${entry.paymentType.label}`); // 例: "運賃引き去り", "チャージ"

  if (entry.entry && entry.exit) {
    // 鉄道利用の場合
    console.log(`区間: ${entry.entry.label} → ${entry.exit.label}`);
  } else if (entry.busCompanyName) {
    // バス利用の場合
    console.log(`バス事業者: ${entry.busCompanyName}`);
  }

  if (entry.amount !== null && entry.amount !== undefined) {
    console.log(`利用額: ¥${entry.amount.toLocaleString()}`);
  }
  console.log(`残高: ¥${entry.balance.toLocaleString()}`);
}
```

#### 駅名の自動補完について (`StationRef`)

本プラグインには全国の JR・私鉄・地下鉄の線区コード・駅順コードの辞書が組み込まれています。

- `label`: 解決された駅名（例: `"東京"`, `"新宿"`）。
- 地区情報（JR東日本/東海/西日本等）が特定できない場合は、`candidates` に候補一覧が保持され、`label` には候補の駅名が記載されます。

---

## 2. WAON (`WaonCard`)

イオングループの電子マネー「WAON」のデータです。

```typescript
if (result.waon) {
  const card = result.waon;
  console.log(`WAON 残高: ¥${card.balance.toLocaleString()}`);
  if (card.points !== undefined) {
    console.log(`WAON ポイント: ${card.points} pt`);
  }
  if (card.waonNumber) {
    console.log(`WAON 番号: ${card.waonNumber}`);
  }

  for (const h of card.histories) {
    console.log(
      `[${h.dateTime?.date} ${h.dateTime?.time}] 利用額: ¥${h.amount} (残高: ¥${h.balance})`,
    );
  }
}
```

### 主要なプロパティ

| プロパティ   | 型                    | 説明                                  |
| :----------- | :-------------------- | :------------------------------------ |
| `balance`    | `number`              | 現在の残高                            |
| `points`     | `number \| undefined` | センター預かり外のカード内ポイント    |
| `waonNumber` | `string \| undefined` | WAON カード番号（16桁）               |
| `histories`  | `WaonHistoryEntry[]`  | 利用履歴（最大 3 件の決済ペアデータ） |

---

## 3. 楽天Edy (`EdyCard`)

楽天Edyの残高および利用履歴です。

```typescript
if (result.edy) {
  const card = result.edy;
  console.log(`Edy 残高: ¥${card.balance.toLocaleString()}`);
  if (card.edyNumber) {
    console.log(`Edy 番号: ${card.edyNumber}`);
  }

  for (const h of card.histories) {
    console.log(
      `[${h.date} ${h.time}] 取引: ¥${h.amount} (残高: ¥${h.balance})`,
    );
  }
}
```

### 主要なプロパティ

| プロパティ  | 型                     | 説明                        |
| :---------- | :--------------------- | :-------------------------- |
| `balance`   | `number`               | 現在の残高                  |
| `edyNumber` | `string \| undefined`  | 16桁の Edy 番号             |
| `histories` | `EmoneyHistoryEntry[]` | 直近の取引履歴（最大 6 件） |

---

## 4. nanaco (`NanacoCard`)

セブン&アイ・ホールディングスの電子マネー「nanaco」のデータです。

```typescript
if (result.nanaco) {
  const card = result.nanaco;
  console.log(`nanaco 残高: ¥${card.balance.toLocaleString()}`);
  if (card.points !== undefined) {
    console.log(`nanaco ポイント: ${card.points} pt`);
  }
  if (card.nanacoNumber) {
    console.log(`nanaco 番号: ${card.nanacoNumber}`);
  }

  for (const h of card.histories) {
    console.log(
      `[${h.date} ${h.time}] 取引: ¥${h.amount} (残高: ¥${h.balance})`,
    );
  }
}
```

### 主要なプロパティ

| プロパティ     | 型                     | 説明                        |
| :------------- | :--------------------- | :-------------------------- |
| `balance`      | `number`               | 現在の残高                  |
| `points`       | `number \| undefined`  | nanaco ポイント             |
| `nanacoNumber` | `string \| undefined`  | 16桁の nanaco 番号          |
| `histories`    | `EmoneyHistoryEntry[]` | 直近の取引履歴（最大 5 件） |

---

## 5. QUICPay (`QuicpayCard`)

QUICPay は後払い式（ポストペイ）規格であり、カード内に公開された無鍵残高サービスが存在しません。本プラグインでは、専用システムコード（`0x04C1`）に対する応答を検出し、QUICPay カードであることの識別を行います。

```typescript
if (result.quicpay) {
  console.log("QUICPay カードが検出されました");
  console.log(`IDm: ${result.quicpay.idm}`);
}
```

---

## 6. FeliCa Lite / Lite-S (`LiteCard`)

FeliCa Lite および Lite-S（システムコード `0x88B4`）の読み取りに対応しています。ユーザーが自由に読み書きできる公開無鍵領域であるスクラッチパッド（S_PAD0 〜 S_PAD13）のバイナリデータを取得できます。

```typescript
if (result.lite) {
  console.log("FeliCa Lite / Lite-S カードが検出されました");
  console.log(`IDm: ${result.lite.idm}`);

  // S_PAD の各ブロック (0x00 ~ 0x0D) の 16進文字列
  for (const [blockIndex, hexData] of Object.entries(result.lite.sPad)) {
    console.log(`S_PAD[${blockIndex}]: ${hexData}`);
  }
}
```

---

次のステップ: **[低レベル API ガイド](Low-Level-API)** で、任意のサービスコードやブロックを直接指定して読み取る高度な方法を確認しましょう。
