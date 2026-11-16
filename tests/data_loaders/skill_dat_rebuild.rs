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
fn skill_dat_has_all_376_records() {
    let recs = load();
    assert_eq!(
        recs.len(),
        376,
        "Skill.Dat phai co 376 record (dung bang skills.json)"
    );
}

#[test]
fn skill_dat_contains_hardcoded_gate_skills() {
    let recs = load();
    let by_id: std::collections::HashMap<u16, &str> =
        recs.iter().map(|r| (r.id, r.name.as_str())).collect();
    // 4 "Tam phap goc" ma FUN_006538BC chan khi player+0x4B0 == 0
    for (id, name) in [
        (10020u16, "Địa Động Quyết"),
        (11020, "Băng Tâm Quyết"),
        (12020, "Hỏa Dương Quyết"),
        (13019, "Phong Ngạo Quyết"),
    ] {
        assert_eq!(
            by_id.get(&id).copied(),
            Some(name),
            "id {id} phai la '{name}' (client hard-code)"
        );
    }
}

#[test]
fn skill_dat_contains_four_job_branches() {
    let recs = load();
    let by_id: std::collections::HashMap<u16, &str> =
        recs.iter().map(|r| (r.id, r.name.as_str())).collect();
    // 4 nhanh nghe, thu tu client: Ba - Hiep - Hien - Tien
    let branches: [(u16, &str); 4] = [
        (14039, "Bá khí"),
        (14043, "Anh khí hộ thân"),
        (14047, "Dùng kế"),
        (14051, "Tiên khiếu"),
    ];
    for (id, name) in branches {
        assert_eq!(
            by_id.get(&id).copied(),
            Some(name),
            "id {id} phai la '{name}'"
        );
    }
    // goc chung
    assert_eq!(by_id.get(&14038).copied(), Some("Đại chu thiên"));
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
