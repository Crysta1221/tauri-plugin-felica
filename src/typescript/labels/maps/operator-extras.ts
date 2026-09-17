/**
 * Line-level operators from public gate-code surveys (ysrl station-code tables)
 * for lines missing from ekicode.csv. Overlay on generated line operators.
 */
export const CYBERNETICS_LINE_OPERATOR_EXTRAS: Readonly<Record<string, string>> = {
  "0-90": "WILLER TRAINS",
  "0-220": "アルピコ交通",
  "1-203": "樽見鉄道",
  "1-204": "伊賀鉄道",
  "1-212": "岳南電車",
  "1-214": "大井川鐵道",
  "1-216": "四日市あすなろう鉄道",
  "2-207": "京福電気鉄道",
  "2-208": "京福電気鉄道",
  "2-210": "叡山電鉄",
  "2-223": "近江鉄道",
  "2-224": "近江鉄道",
  "3-220": "沖縄都市モノレール",
  "3-228": "肥薩おれんじ鉄道",
  "3-230": "北九州高速鉄道",
};

/** Official / common display names for generated legal company names. */
export const OPERATOR_DISPLAY_NAMES: Readonly<Record<string, string>> = {
  東日本旅客鉄道: "JR東日本",
  東海旅客鉄道: "JR東海",
  西日本旅客鉄道: "JR西日本",
  北海道旅客鉄道: "JR北海道",
  四国旅客鉄道: "JR四国",
  九州旅客鉄道: "JR九州",
  JR九州: "JR九州",
  大阪市交通局: "Osaka Metro",
  大阪府都市開発: "泉北高速鉄道",
  北近畿タンゴ鉄道: "WILLER TRAINS",
  東京地下鉄: "東京メトロ",
};
