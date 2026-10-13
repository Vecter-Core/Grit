//! `rig.lock`: băm nội dung từng gói (chuỗi cung ứng: gói bị sửa nội dung ⇒ khoá không khớp ⇒ rig từ chối build).
//! Băm = SHA-256 của "rig-hash-v1" + `rig.toml` (đã chuẩn hoá LF) + từng tệp `src/*.gt` theo tên (tên, độ dài, nội dung).
use crate::project::{Package, Project};
use gt_driver::sha256::sha256_hex;
use std::collections::BTreeMap;

#[must_use]
pub fn package_hash(p: &Package) -> String {
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(b"rig-hash-v1\n");
    let mut add = |name: &str, content: &str| {
        data.extend_from_slice(name.as_bytes());
        data.push(0);
        data.extend_from_slice(content.len().to_string().as_bytes());
        data.push(0);
        data.extend_from_slice(content.as_bytes());
        data.push(b'\n');
    };
    add("rig.toml", &p.manifest_text);
    for (n, t) in &p.files {
        add(&format!("src/{n}"), t);
    }
    sha256_hex(&data)
}

/// Nội dung `rig.lock` đúng của dự án (tất định: gói sắp theo tên).
#[must_use]
pub fn render(project: &Project) -> String {
    let mut pk: Vec<&Package> = project.pkgs.iter().collect();
    pk.sort_by(|a, b| a.name.cmp(&b.name));
    let mut s = String::from(
        "# rig.lock — tạo bởi `rig lock`; KHÔNG sửa tay. Băm nội dung từng gói; lệch ⇒ rig từ chối build.\nversion = 1\n",
    );
    for p in pk {
        s.push_str("\n[[package]]\n");
        s.push_str(&format!("name = \"{}\"\n", p.name));
        s.push_str(&format!("version = \"{}\"\n", p.version));
        s.push_str(&format!(
            "source = \"{}\"\n",
            if p.is_root { "root" } else { "path" }
        ));
        let mut ds: Vec<&str> = p.deps.iter().map(|(n, _)| n.as_str()).collect();
        ds.sort_unstable();
        s.push_str(&format!(
            "deps = [{}]\n",
            ds.iter()
                .map(|d| format!("\"{d}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        s.push_str(&format!("hash = \"sha256:{}\"\n", package_hash(p)));
    }
    s
}

fn hashes(text: &str) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    let mut cur: Option<String> = None;
    for l in text.lines() {
        if let Some(n) = l.strip_prefix("name = \"").and_then(|r| r.strip_suffix('"')) {
            cur = Some(n.to_string());
        } else if let Some(h) = l.strip_prefix("hash = \"").and_then(|r| r.strip_suffix('"'))
            && let Some(n) = cur.take()
        {
            m.insert(n, h.to_string());
        }
    }
    m
}

/// So `rig.lock` đang có với bản đúng; lỗi nêu GÓI nào lệch.
pub fn verify(project: &Project, existing: &str) -> Result<(), String> {
    let want = render(project);
    let have = existing.replace("\r\n", "\n");
    if want == have {
        return Ok(());
    }
    let (w, h) = (hashes(&want), hashes(&have));
    let mut why = Vec::new();
    for (n, hw) in &w {
        match h.get(n) {
            None => why.push(format!("gói `{n}` chưa có trong rig.lock")),
            Some(hh) if hh != hw => why.push(format!(
                "gói `{n}` đã đổi nội dung (lock {} ≠ hiện tại {})",
                &hh[..hh.len().min(23)],
                &hw[..hw.len().min(23)]
            )),
            _ => {}
        }
    }
    for n in h.keys() {
        if !w.contains_key(n) {
            why.push(format!("rig.lock còn gói `{n}` không còn trong dự án"));
        }
    }
    if why.is_empty() {
        why.push("khác ở metadata (phiên bản/phụ thuộc/định dạng)".to_string());
    }
    Err(format!(
        "rig.lock không khớp dự án: {}. Nếu thay đổi là có chủ đích, chạy `rig lock`.",
        why.join("; ")
    ))
}
