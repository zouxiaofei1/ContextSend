//! 设备随机命名：从内置中文词表组合「形容词 + 名词」，生成默认设备名（如「晨雾微风」）。
//!
//! 仅用于初次默认名，用户可随时改名。

use rand::seq::SliceRandom;
use rand::thread_rng;

/// 形容词 / 修饰词。
const ADJECTIVES: &[&str] = &[
    "晨雾", "微风", "静水", "流云", "暖阳", "幽谷", "疏星", "清露", "远山", "归舟", "落霞", "听雨",
    "踏雪", "望月", "拾光", "寻芳", "煮茶", "观潮", "栖林", "枕书",
];

/// 名词 / 意象。
const NOUNS: &[&str] = &[
    "微风", "归鸟", "山岚", "孤舟", "晚钟", "新荷", "竹影", "松涛", "海月", "星河", "灯火", "桥影",
    "渡口", "野渡", "苔痕", "檐角", "回廊", "石阶", "扁舟", "栖鹤",
];

/// 生成一个随机设备名，形如「形容词+名词」。
pub fn random_name() -> String {
    let mut rng = thread_rng();
    let adj = ADJECTIVES.choose(&mut rng).copied().unwrap_or("无名");
    let noun = NOUNS.choose(&mut rng).copied().unwrap_or("设备");
    format!("{adj}{noun}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_name_is_non_empty_chinese() {
        let name = random_name();
        assert!(!name.is_empty());
        // 至少包含 4 个汉字（两个双字词）。
        assert!(name.chars().count() >= 4);
    }
}
