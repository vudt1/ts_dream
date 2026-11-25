//! Kiểm tra `Data/Skill.Dat` — file kỹ năng của client `aLogin.exe`.
//!
//! Lịch sử (2026-10-22): `Data/Skill.Dat` trong repo chỉ có **352 record** và
//! dùng **không gian id riêng** (không phải id trên wire). File đúng cho build
//! `aLogin.exe` là bản **376 record** dựng lại từ `Data/skills.json`
//! (xem `client_skills_handler/tools/build_skill_dat.py`).
//!
//! Bằng chứng `skills.json` là `Skill.Dat` đúng bản `aLogin.exe`:
//! cả 33 id mà binary hard-code đều resolve đúng, và cây `require_sk` không
//! có node trôi nào.
//!
//! Id **trên wire** = id trong `Data/skills.json` (khớp các hằng số hard-code
//! `0x2724 / 0x3E8 / 0x3E8 / 0x3E7` mà `FUN_006538BC` dùng để chặn 4 kỹ năng
//! "Tâm pháp gốc": 10020 / 11020 / 12020 / 13019).

use ts_dream::data::loaders::skill_pc::SkillDatLoaderPc;

fn load() -> Vec<ts_dream::data::loaders::skill_pc::PcSkillDef> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/Data/Skill.Dat");
    let bytes = std::fs::read(path).expect("doc Data/Skill.Dat");
    let (_binary, pc) = SkillDatLoaderPc::load_with_pc(&bytes).expect("parse Skill.Dat");
    pc.into_values().collect()
}

#[test]
fn skill_dat_has_all_425_records_synced_with_skills_json() {
    let recs = load();
    assert_eq!(
        recs.len(),
        425,
        "Skill.Dat phai co 425 record (dong bo voi Data/skills.json)"
    );

    let json_path = concat!(env!("CARGO_MANIFEST_DIR"), "/Data/skills.json");
    let json_text = std::fs::read_to_string(json_path).expect("doc Data/skills.json");
    let json_val: Vec<serde_json::Value> =
        serde_json::from_str(&json_text).expect("parse Data/skills.json");
    let json_ids: std::collections::HashSet<u16> = json_val
        .iter()
        .filter_map(|v| v.get("id").and_then(|id| id.as_u64()).map(|id| id as u16))
        .collect();
    let dat_ids: std::collections::HashSet<u16> = recs.iter().map(|r| r.id).collect();
    assert_eq!(dat_ids, json_ids, "Skill.Dat va skills.json phai khop 100% tap ID");
}

#[test]
fn skill_dat_contains_hardcoded_gate_skills() {
    let recs = load();
    let by_id: std::collections::HashMap<u16, &str> =
        recs.iter().map(|r| (r.id, r.name.as_str())).collect();
    // 4 "Tam phap goc" ma FUN_006538BC chan khi player+0x4B0 == 0
    for id in [10020u16, 11020, 12020, 13019] {
        let name = by_id.get(&id).copied();
        assert!(
            name.is_some_and(|n| !n.is_empty()),
            "id {id} phai ton tai trong Skill.Dat (client hard-code)"
        );
    }
}

#[test]
fn skill_dat_contains_four_job_branches() {
    let recs = load();
    let by_id: std::collections::HashMap<u16, &ts_dream::data::loaders::skill_pc::PcSkillDef> =
        recs.iter().map(|r| (r.id, r)).collect();
    // goc chung
    assert!(
        by_id.get(&14038).is_some_and(|r| !r.name.is_empty()),
        "id 14038 (Dai chu thien) phai ton tai"
    );
    // 4 nhanh nghe, thu tu client: Ba - Hiep - Hien - Tien
    for id in [14039u16, 14043, 14047, 14051] {
        let rec = by_id.get(&id);
        assert!(
            rec.is_some_and(|r| !r.name.is_empty() && r.require_sk == 14038),
            "id {id} phai ton tai va require_sk == 14038"
        );
    }
}

#[test]
fn skill_dat_require_sk_forest_has_no_orphan() {
    let recs = load();
    let ids: std::collections::HashSet<u16> = recs.iter().map(|r| r.id).collect();
    for r in &recs {
        if r.require_sk != 0 {
            assert!(
                ids.contains(&r.require_sk),
                "id {} ({}) require_sk={} khong ton tai",
                r.id,
                r.name,
                r.require_sk
            );
        }
    }
}
