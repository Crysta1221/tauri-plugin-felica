# 低レベル API ガイド

高レベル API（`FelicaReader.scan()`）で自動解釈されない独自仕様の FeliCa カードを解析する場合や、特定のサービスコード・ブロック番号をピンポイントで読み出したい場合は、低レベル API を使用します。

---

## 2 つのアプローチ

低レベルアクセスには、手軽な**ワンショット読み取り**と、詳細な制御が可能な**カード対話セッション**の 2 つの方法があります。

1. **ワンショット読み取り (`reader.read()`)**
   指定したシステム・サービス・ブロックのデータを 1 回の呼び出しで一括取得します。
2. **カード対話セッション (`reader.poll()` → `FelicaCard`)**
   カードとの接続を維持したまま、サービスの探索や検証、特定ブロックの読み取りを対話的に実行します。

---

## 1. ワンショット読み取り (`reader.read()`)

あらかじめ読み取り対象のシステムコード、サービスコード、ブロック番号が判明している場合に最適です。

```typescript
import { FelicaReader } from "tauri-plugin-felica-api";

const readers = await FelicaReader.list();
const reader = await FelicaReader.connect({ id: readers[0].id });

try {
  // 交通系ICの属性情報 (0x008B) のブロック 0 を直接読む
  const result = await reader.read([
    {
      systemCode: 0x0003, // サイバネ規格システムコード
      serviceCode: 0x008b, // 属性情報サービスコード (RO無鍵)
      blocks: [0],
    },
  ]);

  for (const block of result.blocks) {
    console.log(`Block ${block.blockIndex}: ${block.dataHex}`);
  }

  // 読み取りエラーの確認
  for (const err of result.errors) {
    console.error(`Block ${err.blockIndex} 読み取り失敗: ${err.reason}`);
  }
} finally {
  await reader.disconnect();
}
```

---

## 2. カード対話セッション (`reader.poll()`)

`reader.poll()` を呼び出すと、カードを捕捉して `FelicaCard` インスタンスが返されます。
このインスタンスを通じて、カードに対する詳細な FeliCa コマンドを発行できます。

```typescript
const card = await reader.poll(); // 省略時はシステム 0 (ワイルドカード) でポーリング
```

> [!IMPORTANT]
> `FelicaCard` を取得している間はリーダーの排他権を保持します。
> 操作が完了したら、必ず `await card.release()` を呼び出して解放してください。

### サービスコードの存在確認 (`card.requestService()`)

指定したサービスコードがカード内に存在するか（暗号鍵が必要か / 無鍵か）を確認します。

```typescript
// 交通系ICの主要サービスコードの存在を確認
const statusList = await card.requestService([0x008b, 0x090f, 0x108f]);

for (const status of statusList) {
  console.log(
    `サービス 0x${status.serviceCode.toString(16)}: 存在=${status.exists}, 鍵バージョン=0x${status.keyVersion.toString(16)}`
  );
}
```

- `keyVersion === 0xFFFF` の場合、そのサービスコードはカード内に存在しないことを示します。

### サービスノードの探索 (`card.searchServices()`)

カード内に登録されているエリアやサービスコードを順番に列挙します。

```typescript
// カード内の全サービスノードを探索
const nodes = await card.searchServices({ from: 0, max: 32 });

for (const node of nodes) {
  console.log(
    `[Index ${node.index}] 種別: ${node.kind}, コード: 0x${node.code.toString(16)}`
  );
}
```

### ブロックの読み取り (`card.read()`)

選択中のシステムコードに対して、指定したサービスコードのブロックを読み取ります。

```typescript
// サービス 0x090F (取引履歴) のブロック 0〜2 を読み取り
const result = await card.read(0x090f, [0, 1, 2]);

for (const block of result.blocks) {
  console.log(`ブロック ${block.blockIndex} データ: ${block.dataHex}`);
}
```

### システムコードの切り替え (`card.selectSystem()`)

1 枚の物理カード内に複数のシステム（例: 交通系 0x0003 と 共通領域 0xFE00）が存在する場合、システムコードを切り替えて通信対象を変更できます。

```typescript
// システム 0xFE00 (電子マネー共通領域) に切り替え
const newIdentity = await card.selectSystem(0xfe00);
console.log(`切り替え後の IDm: ${newIdentity.idm}`);
console.log(`切り替え後の PMm: ${newIdentity.pmm}`);
```

### ハンドルの解放 (`card.release()`)

```typescript
await card.release();
```

---

## 総合サンプル：低レベルでのカード調査

```typescript
import { FelicaReader } from "tauri-plugin-felica-api";

async function inspectCard() {
  const readers = await FelicaReader.list();
  if (readers.length === 0) return;

  const reader = await FelicaReader.connect({ id: readers[0].id });

  try {
    console.log("カードをかざしてください...");
    const card = await reader.poll();

    try {
      console.log(`IDm: ${card.idm}`);
      console.log(`PMm: ${card.pmm}`);
      console.log(
        `検出されたシステムコード: ${card.systemCodes
          .map((c) => `0x${c.toString(16).padStart(4, "0")}`)
          .join(", ")}`
      );

      // サービス探索
      const nodes = await card.searchServices();
      console.log(`発見されたサービス/エリア数: ${nodes.length}`);

    } finally {
      // 必ずカードを解放
      await card.release();
    }
  } finally {
    await reader.disconnect();
  }
}
```

---

次のステップ: **[対応ハードウェアと環境](Supported-Hardware)** で、利用可能なリーダーやチップセットの仕様を確認しましょう。
