//! Lucky-box random rewards + fixed multi-item packs — C# case 15
//! (`Client.cs:4041-4759`). Each box rolls a reward via the injected
//! `.NET`-compatible `DotNetRandom` (C# `new Random()` per use), adds it with
//! `HomdoAddItem` (`1706` reward frame), then consumes with `HomdoUseHPSPFAI`.
//! The 29/28/19-entry switch tables are transcribed verbatim (bug-for-bug).

use super::UseCtx;
use crate::battle::rng::DotNetRandom;

/// Roll one lucky-box reward. Returns `(item_id, count)`.
async fn roll(ctx: &mut UseCtx<'_>) {
    let id = ctx.id;
    let rng: &mut DotNetRandom = ctx.rng;
    match id {
        // C# 99999 is unreachable here: item ids are u16 (max 65535) and 99999
        // is not present in the dataset, so no inventory slot can hold it.
        // 46129: Next(0,4); 0|4 → Next(0,7), else Next(4,7) → 57005+n*100+r.
        46129 => {
            let n = rng.next_range(0, 4);
            let r = if n == 0 || n == 4 {
                rng.next_range(0, 7)
            } else {
                rng.next_range(4, 7)
            };
            ctx.add_reward((57005 + n * 100 + r) as u16, 1).await;
            ctx.consume().await;
        }
        // 46627: Next(2,2)=2 always → Next(1,9) → 57700+200+r.
        46627 => {
            let n = rng.next_range(2, 2);
            let r = rng.next_range(1, 9);
            ctx.add_reward((57700 + n * 100 + r) as u16, 1).await;
            ctx.consume().await;
        }
        // 46646: Next(2); → Next(1,5) → 51289+r (both branches equal).
        46646 => {
            let _ = rng.next_max(2);
            let r = rng.next_range(1, 5);
            ctx.consume().await;
            ctx.add_reward((51289 + r) as u16, 1).await;
        }
        // 46935: Next(2); → Next(1,5) → 51270+r (both branches equal).
        46935 => {
            let _ = rng.next_max(2);
            let r = rng.next_range(1, 5);
            ctx.consume().await;
            ctx.add_reward((51270 + r) as u16, 1).await;
        }
        // 46934: Next(4); 1|2 → Next(1,3)→47050+r; 3 → Next(1,2)→47071+r;
        //        else Next(1,2)→47035+r.
        46934 => {
            let n = rng.next_max(4);
            let reward = if n == 1 || n == 2 {
                47050 + rng.next_range(1, 3)
            } else if n == 3 {
                47071 + rng.next_range(1, 2)
            } else {
                47035 + rng.next_range(1, 2)
            };
            ctx.add_reward(reward as u16, 1).await;
            ctx.consume().await;
        }
        // 46395: Next(29) → 29-entry table.
        46395 => {
            let n = rng.next_max(29);
            let reward = match n {
                1 => 47316,
                2 => 47040,
                4 => 47256,
                5 => 47216,
                6 => 47155,
                7 => 47147,
                8 => 47127,
                9 => 47053,
                10 => 47117,
                11 => 47088,
                12 => 47198,
                13 => 47056,
                14 => 47086,
                15 => 47207,
                16 => 47037,
                17 => 47135,
                18 => 47141,
                19 => 47091,
                20 => 47080,
                21 => 47159,
                22 => 47113,
                23 => 47033,
                24 => 47171,
                25 => 47188,
                26 => 47167,
                27 => 47087,
                28 => 47224,
                3 => 47175,
                _ => 47280,
            };
            ctx.add_reward(reward as u16, 1).await;
            ctx.consume().await;
        }
        // 46648: Next(3); 1→47443, 2→47444, _→47445.
        46648 => {
            let n = rng.next_max(3);
            let reward = match n {
                1 => 47443,
                2 => 47444,
                _ => 47445,
            };
            ctx.add_reward(reward as u16, 1).await;
            ctx.consume().await;
        }
        // 46396: Next(29) → 29-entry table.
        46396 => {
            let n = rng.next_max(29);
            let reward = match n {
                1 => 47065,
                2 => 47142,
                4 => 47290,
                5 => 47328,
                6 => 47289,
                7 => 47043,
                8 => 47070,
                9 => 47067,
                10 => 47225,
                11 => 47076,
                12 => 47160,
                13 => 47168,
                14 => 47158,
                15 => 47079,
                16 => 47204,
                17 => 47143,
                18 => 47025,
                19 => 47132,
                20 => 47215,
                21 => 47150,
                22 => 47239,
                23 => 47061,
                24 => 47110,
                25 => 47128,
                26 => 47230,
                27 => 47071,
                28 => 47149,
                3 => 47158,
                _ => 47225,
            };
            ctx.add_reward(reward as u16, 1).await;
            ctx.consume().await;
        }
        // 46397: Next(29) → 29-entry table.
        46397 => {
            let n = rng.next_max(29);
            let reward = match n {
                1 => 47024,
                2 => 47032,
                4 => 47183,
                5 => 47019,
                6 => 47261,
                7 => 47220,
                8 => 47111,
                9 => 47022,
                10 => 47152,
                11 => 47169,
                12 => 47173,
                13 => 47030,
                14 => 47161,
                15 => 47105,
                16 => 47129,
                17 => 47310,
                18 => 47028,
                19 => 47115,
                20 => 47034,
                21 => 47156,
                22 => 47041,
                23 => 47151,
                24 => 47029,
                25 => 47031,
                26 => 47049,
                27 => 47097,
                28 => 47026,
                3 => 47019,
                _ => 47169,
            };
            ctx.add_reward(reward as u16, 1).await;
            ctx.consume().await;
        }
        // 46398: Next(29) → 29-entry table.
        46398 => {
            let n = rng.next_max(29);
            let reward = match n {
                1 => 47186,
                2 => 47094,
                4 => 47112,
                5 => 47138,
                6 => 47054,
                7 => 47058,
                8 => 47263,
                9 => 47148,
                10 => 47095,
                11 => 47116,
                12 => 47153,
                13 => 47222,
                14 => 47185,
                15 => 47063,
                16 => 47170,
                17 => 47057,
                18 => 47047,
                19 => 47050,
                20 => 47093,
                21 => 47035,
                22 => 47312,
                23 => 47069,
                24 => 47048,
                25 => 47157,
                26 => 47045,
                27 => 47245,
                28 => 47107,
                3 => 47344,
                _ => 47116,
            };
            ctx.add_reward(reward as u16, 1).await;
            ctx.consume().await;
        }
        // 46920: Next(19) → 19-entry table.
        46920 => {
            let n = rng.next_max(19);
            let reward = match n {
                1 => 22437,
                2 => 20739,
                4 => 22738,
                5 => 19739,
                6 => 21232,
                7 => 20437,
                8 => 21437,
                9 => 21738,
                10 => 19437,
                11 => 22438,
                12 => 20740,
                13 => 22739,
                14 => 19740,
                15 => 21236,
                16 => 20438,
                17 => 21438,
                18 => 21739,
                3 => 19438,
                _ => 19438,
            };
            ctx.add_reward(reward as u16, 1).await;
            ctx.consume().await;
        }
        // 46090: Next(28) → 28-entry table.
        46090 => {
            let n = rng.next_max(28);
            let reward = match n {
                1 => 57001,
                2 => 57002,
                4 => 57003,
                5 => 57004,
                6 => 57101,
                7 => 57102,
                8 => 57103,
                9 => 57104,
                10 => 57201,
                11 => 57202,
                12 => 57203,
                13 => 57204,
                14 => 57501,
                15 => 57502,
                16 => 57503,
                17 => 57504,
                18 => 57601,
                19 => 57604,
                20 => 57701,
                21 => 57702,
                22 => 57703,
                23 => 57704,
                24 => 57801,
                25 => 57802,
                26 => 57803,
                27 => 57804,
                3 => 57602,
                _ => 57603,
            };
            ctx.add_reward(reward as u16, 1).await;
            ctx.consume().await;
        }
        // 46137: Next(4); 1→22941, 2→22942, 3→22943, _→22944.
        46137 => {
            let n = rng.next_max(4);
            let reward = match n {
                1 => 22941,
                2 => 22942,
                3 => 22943,
                _ => 22944,
            };
            ctx.add_reward(reward as u16, 1).await;
            ctx.consume().await;
        }
        // 46138: Next(4); 1→15046, 2→15047, 3→15048, _→15049.
        46138 => {
            let n = rng.next_max(4);
            let reward = match n {
                1 => 15046,
                2 => 15047,
                3 => 15048,
                _ => 15049,
            };
            ctx.add_reward(reward as u16, 1).await;
            ctx.consume().await;
        }
        // 46139: Next(4); 1→20064, 2→20065, 3→20066, _→20067.
        46139 => {
            let n = rng.next_max(4);
            let reward = match n {
                1 => 20064,
                2 => 20065,
                3 => 20066,
                _ => 20067,
            };
            ctx.add_reward(reward as u16, 1).await;
            ctx.consume().await;
        }
        // 46383: Next(3); 1→30084, 2→30085, _→30083, count 9.
        46383 => {
            let n = rng.next_max(3);
            let reward = match n {
                1 => 30084,
                2 => 30085,
                _ => 30083,
            };
            ctx.add_reward(reward as u16, 9).await;
            ctx.consume().await;
        }
        // 46930: Next(2); 1|3 → Next(1,5)→62600+r; else Next(1,5)→62700+n*200+r.
        46930 => {
            let n = rng.next_max(2);
            let r = rng.next_range(1, 5);
            let reward = if n == 1 || n == 3 {
                62600 + r
            } else {
                62700 + n * 200 + r
            };
            ctx.consume().await;
            ctx.add_reward(reward as u16, 1).await;
        }
        // 46929: Next(2); 1|3 → Next(1,5)→62400+r; else Next(1,5)→62500+n*200+r.
        46929 => {
            let n = rng.next_max(2);
            let r = rng.next_range(1, 5);
            let reward = if n == 1 || n == 3 {
                62400 + r
            } else {
                62500 + n * 200 + r
            };
            ctx.consume().await;
            ctx.add_reward(reward as u16, 1).await;
        }
        // 46928: Next(2); 1|2 → Next(1,5)→62200+r; else Next(1,5)→62300+n*100+r.
        46928 => {
            let n = rng.next_max(2);
            let r = rng.next_range(1, 5);
            let reward = if n == 1 || n == 2 {
                62200 + r
            } else {
                62300 + n * 100 + r
            };
            ctx.consume().await;
            ctx.add_reward(reward as u16, 1).await;
        }
        // 46927: Next(2); 1|2 → Next(1,5)→62000+r; else Next(1,5)→62100+n*100+r.
        46927 => {
            let n = rng.next_max(2);
            let r = rng.next_range(1, 5);
            let reward = if n == 1 || n == 2 {
                62000 + r
            } else {
                62100 + n * 100 + r
            };
            ctx.consume().await;
            ctx.add_reward(reward as u16, 1).await;
        }
        // 46659: Next(2); 1|2 → Next(1,17)→62866+r; else Next(1,5)→57964+r.
        46659 => {
            let n = rng.next_max(2);
            let reward = if n == 1 || n == 2 {
                62866 + rng.next_range(1, 17)
            } else {
                57964 + rng.next_range(1, 5)
            };
            ctx.consume().await;
            ctx.add_reward(reward as u16, 1).await;
        }
        // 46671: Next(2,2)=2 → Next(1,9)→62674+200+r.
        46671 => {
            let n = rng.next_range(2, 2);
            let r = rng.next_range(1, 9);
            ctx.consume().await;
            ctx.add_reward((62674 + n * 100 + r) as u16, 1).await;
        }
        // 46670: Next(2,2)=2 → Next(1,9)→62666+200+r.
        46670 => {
            let n = rng.next_range(2, 2);
            let r = rng.next_range(1, 9);
            ctx.consume().await;
            ctx.add_reward((62666 + n * 100 + r) as u16, 1).await;
        }
        // 46953: Next(47001,47456) → random item.
        46953 => {
            let reward = rng.next_range(47001, 47456);
            ctx.add_reward(reward as u16, 1).await;
            ctx.consume().await;
        }
        _ => {}
    }
}

/// Handle the lucky-box family. Returns true when the item id was handled.
pub async fn handle(ctx: &mut UseCtx<'_>) -> bool {
    let id = ctx.id;
    let boxes = [
        46129, 46627, 46646, 46935, 46934, 46395, 46648, 46396, 46397, 46398, 46920, 46090,
        46137, 46138, 46139, 46383, 46930, 46929, 46928, 46927, 46659, 46671, 46670, 46953,
    ];
    if !boxes.contains(&id) {
        return fixed_pack(ctx).await;
    }
    roll(ctx).await;
    true
}

/// Fixed multi-item packs (C# 46908-46911, 46900-46907, 46905/46904/46906/46907,
/// 46077, 46197). Returns true when handled.
async fn fixed_pack(ctx: &mut UseCtx<'_>) -> bool {
    let id = ctx.id;
    // (item, count) list, transcribed from Client.cs.
    let pack: &[(u16, u8)] = match id {
        46908 => &[(15054, 1), (21628, 1), (20643, 1), (22941, 1), (19646, 1)],
        46909 => &[(15055, 1), (21629, 1), (20644, 1), (22942, 1), (19647, 1), (19647, 1)],
        46910 => &[(15056, 1), (21630, 1), (20645, 1), (22943, 1), (19648, 1)],
        46911 => &[(15057, 1), (21631, 1), (20646, 1), (22944, 1), (19649, 1)],
        46900 => &[(18963, 1), (20446, 1), (19629, 1), (21444, 1), (22924, 1)],
        46901 => &[(18964, 1), (20447, 1), (19630, 1), (21445, 1), (22925, 1)],
        46902 => &[(18965, 1), (20448, 1), (19631, 1), (21446, 1), (22926, 1)],
        46903 => &[(18966, 1), (20449, 1), (19632, 1), (21447, 1), (22927, 1)],
        46905 => &[(22046, 1), (19233, 1), (20047, 1), (21046, 1)],
        46904 => &[(22045, 1), (19232, 1), (20046, 1), (21045, 1)],
        46906 => &[(22047, 1), (19234, 1), (20048, 1), (21047, 1)],
        46907 => &[(22048, 1), (19235, 1), (20049, 1), (21048, 1)],
        46077 => &[(26456, 5), (26457, 5), (46092, 50), (46169, 50)],
        46197 => &[(26456, 10), (26457, 10)],
        _ => return false,
    };
    for &(item, n) in pack {
        ctx.add_reward(item, n).await;
    }
    ctx.consume().await;
    true
}