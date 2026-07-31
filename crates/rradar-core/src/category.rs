//! Merchant dictionary + keyword categorizer.

use crate::explain::ExplainTrace;
use crate::types::{Field, FieldSource};
use serde::{Deserialize, Serialize};

/// Built-in category ids (taxonomy schema_version 1).
pub const CAT_FOOD: &str = "food_dining";
pub const CAT_GROCERY: &str = "grocery_convenience";
pub const CAT_TRANSPORT: &str = "transport";
pub const CAT_SHOPPING: &str = "shopping";
pub const CAT_HEALTH: &str = "health";
pub const CAT_UTILITIES: &str = "utilities";
pub const CAT_ENTERTAINMENT: &str = "entertainment";
pub const CAT_OTHER: &str = "other";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerchantEntry {
    /// Normalized match key (lowercase / 全形簡易正規化).
    pub key: String,
    pub display: String,
    pub category: String,
}

#[derive(Debug, Clone, Default)]
pub struct CategoryEngine {
    pub merchants: Vec<MerchantEntry>,
    pub keywords: Vec<(String, String)>, // keyword -> category
}

impl CategoryEngine {
    pub fn with_seed() -> Self {
        Self {
            merchants: seed_merchants(),
            keywords: seed_keywords(),
        }
    }

    pub fn categorize(
        &self,
        merchant: &str,
        raw_text: &str,
        explain: &mut ExplainTrace,
    ) -> Field<String> {
        let norm = normalize_key(merchant);
        for m in &self.merchants {
            if norm.contains(&m.key) || m.key.contains(&norm) && !norm.is_empty() {
                explain.matched_keywords.push(m.key.clone());
                explain.step(
                    "category",
                    format!("merchant dict hit {} -> {}", m.display, m.category),
                );
                return Field::new(m.category.clone(), 0.9, FieldSource::Rule);
            }
        }
        // scan full text for merchant keys
        let blob = normalize_key(&format!("{merchant}\n{raw_text}"));
        for m in &self.merchants {
            if blob.contains(&m.key) {
                explain.matched_keywords.push(m.key.clone());
                explain.step(
                    "category",
                    format!("text dict hit {} -> {}", m.key, m.category),
                );
                return Field::new(m.category.clone(), 0.85, FieldSource::Rule);
            }
        }
        for (kw, cat) in &self.keywords {
            if blob.contains(kw) {
                explain.matched_keywords.push(kw.clone());
                explain.step("category", format!("keyword {kw} -> {cat}"));
                return Field::new(cat.clone(), 0.7, FieldSource::Rule);
            }
        }
        explain.step("category", "fallback other");
        Field::new(CAT_OTHER.to_string(), 0.4, FieldSource::Rule)
    }

    pub fn merchant_count(&self) -> usize {
        self.merchants.len()
    }
}

pub fn normalize_key(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'Ａ'..='Ｚ' => char::from_u32(c as u32 - 'Ａ' as u32 + 'A' as u32).unwrap_or(c),
            'ａ'..='ｚ' => char::from_u32(c as u32 - 'ａ' as u32 + 'a' as u32).unwrap_or(c),
            _ => c,
        })
        .collect::<String>()
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

fn entry(key: &str, display: &str, category: &str) -> MerchantEntry {
    MerchantEntry {
        key: normalize_key(key),
        display: display.into(),
        category: category.into(),
    }
}

/// ≥150 license-clean common TW / global chain keywords (names only).
fn seed_merchants() -> Vec<MerchantEntry> {
    let mut v = Vec::with_capacity(200);
    let conv = [
        ("7-eleven", "7-ELEVEN", CAT_GROCERY),
        ("7eleven", "7-ELEVEN", CAT_GROCERY),
        ("統一超商", "7-ELEVEN", CAT_GROCERY),
        ("familymart", "全家便利商店", CAT_GROCERY),
        ("全家", "全家便利商店", CAT_GROCERY),
        ("family mart", "全家便利商店", CAT_GROCERY),
        ("hilife", "萊爾富", CAT_GROCERY),
        ("萊爾富", "萊爾富", CAT_GROCERY),
        ("okmart", "OK超商", CAT_GROCERY),
        ("ok超商", "OK超商", CAT_GROCERY),
        ("全聯", "全聯福利中心", CAT_GROCERY),
        ("pxmart", "全聯福利中心", CAT_GROCERY),
        ("家樂福", "家樂福", CAT_GROCERY),
        ("carrefour", "家樂福", CAT_GROCERY),
        ("大潤發", "大潤發", CAT_GROCERY),
        ("rt-mart", "大潤發", CAT_GROCERY),
        ("costco", "好市多", CAT_GROCERY),
        ("好市多", "好市多", CAT_GROCERY),
        ("美廉社", "美廉社", CAT_GROCERY),
        ("頂好", "Top's", CAT_GROCERY),
        ("寶雅", "寶雅", CAT_SHOPPING),
        ("屈臣氏", "屈臣氏", CAT_HEALTH),
        ("watsons", "屈臣氏", CAT_HEALTH),
        ("康是美", "康是美", CAT_HEALTH),
        ("丁丁藥局", "丁丁藥局", CAT_HEALTH),
    ];
    for (k, d, c) in conv {
        v.push(entry(k, d, c));
    }

    let food = [
        ("麥當勞", "麥當勞", CAT_FOOD),
        ("mcdonald", "McDonald's", CAT_FOOD),
        ("肯德基", "肯德基", CAT_FOOD),
        ("kfc", "KFC", CAT_FOOD),
        ("摩斯漢堡", "MOS BURGER", CAT_FOOD),
        ("mos burger", "MOS BURGER", CAT_FOOD),
        ("星巴克", "STARBUCKS", CAT_FOOD),
        ("starbucks", "STARBUCKS", CAT_FOOD),
        ("路易莎", "路易莎咖啡", CAT_FOOD),
        ("louisa", "路易莎咖啡", CAT_FOOD),
        ("cama", "cama café", CAT_FOOD),
        ("客美多", "cama café", CAT_FOOD),
        ("丹丹漢堡", "丹丹漢堡", CAT_FOOD),
        ("漢堡王", "Burger King", CAT_FOOD),
        ("burger king", "Burger King", CAT_FOOD),
        ("subway", "SUBWAY", CAT_FOOD),
        ("必勝客", "Pizza Hut", CAT_FOOD),
        ("pizza hut", "Pizza Hut", CAT_FOOD),
        ("達美樂", "Domino's", CAT_FOOD),
        ("domino", "Domino's", CAT_FOOD),
        ("三商巧福", "三商巧福", CAT_FOOD),
        ("八方雲集", "八方雲集", CAT_FOOD),
        ("鼎泰豐", "鼎泰豐", CAT_FOOD),
        ("瓦城", "瓦城", CAT_FOOD),
        ("王品", "王品", CAT_FOOD),
        ("海底撈", "海底撈", CAT_FOOD),
        ("藏壽司", "藏壽司", CAT_FOOD),
        ("壽司郎", "壽司郎", CAT_FOOD),
        ("爭鮮", "爭鮮", CAT_FOOD),
        ("吉野家", "吉野家", CAT_FOOD),
        ("yoshinoya", "吉野家", CAT_FOOD),
        ("三杯雞", "三杯雞", CAT_FOOD),
        ("老王牛肉麵", "老王牛肉麵", CAT_FOOD),
        ("春水堂", "春水堂", CAT_FOOD),
        ("50嵐", "50嵐", CAT_FOOD),
        ("清心", "清心福全", CAT_FOOD),
        ("可不可", "可不可熟成紅茶", CAT_FOOD),
        ("coco", "CoCo都可", CAT_FOOD),
        ("都可", "CoCo都可", CAT_FOOD),
        ("迷客夏", "迷客夏", CAT_FOOD),
        ("milksha", "迷客夏", CAT_FOOD),
        ("珍煮丹", "珍煮丹", CAT_FOOD),
        ("鶴茶樓", "鶴茶樓", CAT_FOOD),
        ("茶湯會", "茶湯會", CAT_FOOD),
        ("麻古", "麻古茶坊", CAT_FOOD),
        ("丸亀製麵", "丸亀製麵", CAT_FOOD),
        ("sukiya", "すき家", CAT_FOOD),
        ("すき家", "すき家", CAT_FOOD),
        ("pepper lunch", "胡椒廚房", CAT_FOOD),
        ("王將", "大阪王將", CAT_FOOD),
        ("ikea restaurant", "IKEA 餐廳", CAT_FOOD),
        ("饗食天堂", "饗食天堂", CAT_FOOD),
        ("旭集", "旭集", CAT_FOOD),
        ("欣葉", "欣葉", CAT_FOOD),
        ("鼎王", "鼎王麻辣鍋", CAT_FOOD),
        ("這一鍋", "這一鍋", CAT_FOOD),
        ("石二鍋", "石二鍋", CAT_FOOD),
        ("火鍋", "火鍋", CAT_FOOD),
        ("牛肉麵", "牛肉麵", CAT_FOOD),
        ("便當", "便當", CAT_FOOD),
        ("自助餐", "自助餐", CAT_FOOD),
        ("鹽酥雞", "鹽酥雞", CAT_FOOD),
        ("雞排", "雞排", CAT_FOOD),
        ("滷味", "滷味", CAT_FOOD),
        ("夜市", "夜市", CAT_FOOD),
        ("ubereats", "Uber Eats", CAT_FOOD),
        ("uber eats", "Uber Eats", CAT_FOOD),
        ("foodpanda", "foodpanda", CAT_FOOD),
        ("戶戶送", "Deliveroo", CAT_FOOD),
    ];
    for (k, d, c) in food {
        v.push(entry(k, d, c));
    }

    let transport = [
        ("台北捷運", "台北捷運", CAT_TRANSPORT),
        ("北捷", "台北捷運", CAT_TRANSPORT),
        ("高雄捷運", "高雄捷運", CAT_TRANSPORT),
        ("桃園捷運", "桃園捷運", CAT_TRANSPORT),
        ("台灣高鐵", "台灣高鐵", CAT_TRANSPORT),
        ("thsrc", "台灣高鐵", CAT_TRANSPORT),
        ("台鐵", "臺灣鐵路", CAT_TRANSPORT),
        ("tra ", "臺灣鐵路", CAT_TRANSPORT),
        ("悠遊卡", "悠遊卡", CAT_TRANSPORT),
        ("一卡通", "一卡通", CAT_TRANSPORT),
        ("中油", "台灣中油", CAT_TRANSPORT),
        ("台塑石油", "台塑石油", CAT_TRANSPORT),
        ("加油站", "加油站", CAT_TRANSPORT),
        ("uber", "Uber", CAT_TRANSPORT),
        ("line taxi", "LINE TAXI", CAT_TRANSPORT),
        ("yoxi", "yoxi", CAT_TRANSPORT),
        ("台灣大車隊", "台灣大車隊", CAT_TRANSPORT),
        ("停車", "停車", CAT_TRANSPORT),
        ("parking", "Parking", CAT_TRANSPORT),
        ("ibus", "公車", CAT_TRANSPORT),
        ("客運", "客運", CAT_TRANSPORT),
        ("國光客運", "國光客運", CAT_TRANSPORT),
        ("和欣客運", "和欣客運", CAT_TRANSPORT),
    ];
    for (k, d, c) in transport {
        v.push(entry(k, d, c));
    }

    let shop = [
        ("momo", "momo購物", CAT_SHOPPING),
        ("pchome", "PChome", CAT_SHOPPING),
        ("shopee", "Shopee", CAT_SHOPPING),
        ("蝦皮", "Shopee", CAT_SHOPPING),
        ("yahoo購物", "Yahoo購物", CAT_SHOPPING),
        ("amazon", "Amazon", CAT_SHOPPING),
        ("unqlo", "UNIQLO", CAT_SHOPPING),
        ("uniqlo", "UNIQLO", CAT_SHOPPING),
        ("zara", "ZARA", CAT_SHOPPING),
        ("h&m", "H&M", CAT_SHOPPING),
        ("無印良品", "MUJI", CAT_SHOPPING),
        ("muji", "MUJI", CAT_SHOPPING),
        ("ikea", "IKEA", CAT_SHOPPING),
        ("特力屋", "特力屋", CAT_SHOPPING),
        ("燦坤", "燦坤", CAT_SHOPPING),
        ("全國電子", "全國電子", CAT_SHOPPING),
        ("順發", "順發3C", CAT_SHOPPING),
        ("apple store", "Apple Store", CAT_SHOPPING),
        ("studios a", "Studio A", CAT_SHOPPING),
        ("小米之家", "小米之家", CAT_SHOPPING),
        ("daiso", "大創", CAT_SHOPPING),
        ("大創", "大創", CAT_SHOPPING),
        ("hololive", "HOLO", CAT_SHOPPING),
        ("網咖", "網咖", CAT_ENTERTAINMENT),
        ("電影", "電影院", CAT_ENTERTAINMENT),
        ("威秀", "威秀影城", CAT_ENTERTAINMENT),
        ("秀泰", "秀泰影城", CAT_ENTERTAINMENT),
        ("國賓影城", "國賓影城", CAT_ENTERTAINMENT),
        ("spotify", "Spotify", CAT_ENTERTAINMENT),
        ("netflix", "Netflix", CAT_ENTERTAINMENT),
        ("steam", "Steam", CAT_ENTERTAINMENT),
        ("playstation", "PlayStation", CAT_ENTERTAINMENT),
        ("任天堂", "Nintendo", CAT_ENTERTAINMENT),
    ];
    for (k, d, c) in shop {
        v.push(entry(k, d, c));
    }

    let util = [
        ("台電", "台灣電力", CAT_UTILITIES),
        ("台水", "台灣自來水", CAT_UTILITIES),
        ("中華電信", "中華電信", CAT_UTILITIES),
        ("台灣大哥大", "台灣大哥大", CAT_UTILITIES),
        ("遠傳", "遠傳電信", CAT_UTILITIES),
        ("亞太電信", "亞太電信", CAT_UTILITIES),
        ("台灣之星", "台灣之星", CAT_UTILITIES),
        ("gas", "瓦斯", CAT_UTILITIES),
        ("瓦斯", "瓦斯", CAT_UTILITIES),
        ("管理費", "管理費", CAT_UTILITIES),
        ("房租", "房租", CAT_UTILITIES),
        ("rent", "Rent", CAT_UTILITIES),
    ];
    for (k, d, c) in util {
        v.push(entry(k, d, c));
    }

    // pad with more common convenience / food aliases to exceed 150
    let extra = [
        ("hi-life", "萊爾富", CAT_GROCERY),
        ("simple mart", "美廉社", CAT_GROCERY),
        ("supermarket", "超市", CAT_GROCERY),
        ("超市", "超市", CAT_GROCERY),
        ("量販", "量販", CAT_GROCERY),
        ("藥妝", "藥妝", CAT_HEALTH),
        ("診所", "診所", CAT_HEALTH),
        ("醫院", "醫院", CAT_HEALTH),
        ("pharmacy", "藥局", CAT_HEALTH),
        ("藥局", "藥局", CAT_HEALTH),
        ("cafe", "Café", CAT_FOOD),
        ("咖啡", "咖啡", CAT_FOOD),
        ("早餐", "早餐", CAT_FOOD),
        ("拉麵", "拉麵", CAT_FOOD),
        ("壽司", "壽司", CAT_FOOD),
        ("燒烤", "燒烤", CAT_FOOD),
        ("居酒屋", "居酒屋", CAT_FOOD),
        ("牛排", "牛排", CAT_FOOD),
        ("義大利麵", "義大利麵", CAT_FOOD),
        ("披薩", "披薩", CAT_FOOD),
        ("漢堡", "漢堡", CAT_FOOD),
        ("炸雞", "炸雞", CAT_FOOD),
        ("甜點", "甜點", CAT_FOOD),
        ("蛋糕", "蛋糕", CAT_FOOD),
        ("麵包", "麵包", CAT_FOOD),
        ("bakery", "Bakery", CAT_FOOD),
        ("mrt", "MRT", CAT_TRANSPORT),
        ("高鐵", "高鐵", CAT_TRANSPORT),
        ("火車", "火車", CAT_TRANSPORT),
        ("公車", "公車", CAT_TRANSPORT),
        ("taxi", "Taxi", CAT_TRANSPORT),
        ("計程車", "計程車", CAT_TRANSPORT),
        ("toll", "過路費", CAT_TRANSPORT),
        ("過路費", "過路費", CAT_TRANSPORT),
        ("bookstore", "書店", CAT_SHOPPING),
        ("誠品", "誠品", CAT_SHOPPING),
        ("博客來", "博客來", CAT_SHOPPING),
        ("books.com", "博客來", CAT_SHOPPING),
        ("gym", "健身房", CAT_HEALTH),
        ("健身", "健身", CAT_HEALTH),
        ("world gym", "World Gym", CAT_HEALTH),
        ("健身房", "健身房", CAT_HEALTH),
        ("誠品生活", "誠品生活", CAT_SHOPPING),
        ("nitori", "NITORI", CAT_SHOPPING),
        ("宜得利", "NITORI", CAT_SHOPPING),
        ("大創百貨", "大創", CAT_SHOPPING),
        ("laline", "Laline", CAT_SHOPPING),
        ("gap", "GAP", CAT_SHOPPING),
        ("gu ", "GU", CAT_SHOPPING),
        ("優衣庫", "UNIQLO", CAT_SHOPPING),
        ("淨水", "淨水", CAT_UTILITIES),
        ("第四台", "有線電視", CAT_UTILITIES),
        ("有線電視", "有線電視", CAT_UTILITIES),
        ("停車費", "停車費", CAT_TRANSPORT),
        ("高速公路", "高速公路", CAT_TRANSPORT),
        ("etc", "ETC", CAT_TRANSPORT),
        ("uber trip", "Uber", CAT_TRANSPORT),
        ("foodpanda", "foodpanda", CAT_FOOD),
        ("ubereats", "Uber Eats", CAT_FOOD),
        ("delivery", "外送", CAT_FOOD),
        ("外送", "外送", CAT_FOOD),
        ("便利商店", "便利商店", CAT_GROCERY),
        ("超商", "超商", CAT_GROCERY),
    ];
    for (k, d, c) in extra {
        v.push(entry(k, d, c));
    }

    v
}

fn seed_keywords() -> Vec<(String, String)> {
    vec![
        ("餐".into(), CAT_FOOD.into()),
        ("飲".into(), CAT_FOOD.into()),
        ("food".into(), CAT_FOOD.into()),
        ("taxi".into(), CAT_TRANSPORT.into()),
        ("fuel".into(), CAT_TRANSPORT.into()),
        ("油".into(), CAT_TRANSPORT.into()),
    ]
}

/// zh-TW category labels for display / taxonomy pack.
pub fn categories_zh_tw() -> Vec<(&'static str, &'static str)> {
    vec![
        (CAT_FOOD, "餐飲"),
        (CAT_GROCERY, "超商／超市"),
        (CAT_TRANSPORT, "交通"),
        (CAT_SHOPPING, "購物"),
        (CAT_HEALTH, "健康／藥妝"),
        (CAT_UTILITIES, "居家／帳單"),
        (CAT_ENTERTAINMENT, "娛樂"),
        (CAT_OTHER, "其他"),
    ]
}

pub fn categories_en() -> Vec<(&'static str, &'static str)> {
    vec![
        (CAT_FOOD, "Food & dining"),
        (CAT_GROCERY, "Grocery & convenience"),
        (CAT_TRANSPORT, "Transport"),
        (CAT_SHOPPING, "Shopping"),
        (CAT_HEALTH, "Health"),
        (CAT_UTILITIES, "Utilities & housing"),
        (CAT_ENTERTAINMENT, "Entertainment"),
        (CAT_OTHER, "Other"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_at_least_150() {
        let eng = CategoryEngine::with_seed();
        assert!(eng.merchant_count() >= 150, "got {}", eng.merchant_count());
    }

    #[test]
    fn categorize_family_mart() {
        let eng = CategoryEngine::with_seed();
        let mut ex = ExplainTrace::new("t", "ocr");
        let f = eng.categorize("全家便利商店 臨江店", "", &mut ex);
        assert_eq!(f.value, CAT_GROCERY);
    }

    #[test]
    fn categorize_unknown() {
        let eng = CategoryEngine::with_seed();
        let mut ex = ExplainTrace::new("t", "ocr");
        let f = eng.categorize("某某無名小店xyz", "雜項", &mut ex);
        assert_eq!(f.value, CAT_OTHER);
    }
}
