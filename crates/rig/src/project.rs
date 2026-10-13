//! Đồ thị gói: đọc `rig.toml`, theo phụ thuộc đường dẫn cục bộ, kiểm tên/vòng/va chạm module, gom nguồn.
use crate::toml::{self, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub dir: PathBuf,
    /// (tên phụ thuộc, đường dẫn đã khai báo)
    pub deps: Vec<(String, String)>,
    /// `src/*.gt`: (tên tệp, nội dung đã chuẩn hoá CRLF→LF), sắp theo tên
    pub files: Vec<(String, String)>,
    /// nội dung `rig.toml` đã chuẩn hoá
    pub manifest_text: String,
    pub is_root: bool,
}

#[derive(Clone, Debug)]
pub struct Project {
    /// gói phụ thuộc trước, gốc cuối (thứ tự tất định: DFS hậu thứ tự theo tên phụ thuộc)
    pub pkgs: Vec<Package>,
}

impl Project {
    #[must_use]
    pub fn root(&self) -> &Package {
        &self.pkgs[self.pkgs.len() - 1]
    }

    /// Mọi tệp nguồn của chương trình, sắp theo tên tệp (tất định). `skip_root_main`: bỏ `main.gt` của gói gốc (dùng cho `rig test`).
    #[must_use]
    pub fn sources(&self, skip_root_main: bool) -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = Vec::new();
        for p in &self.pkgs {
            for (n, t) in &p.files {
                if skip_root_main && p.is_root && n == "main.gt" {
                    continue;
                }
                v.push((n.clone(), t.clone()));
            }
        }
        v.sort();
        v
    }
}

fn valid_pkg_name(s: &str) -> bool {
    let mut cs = s.chars();
    matches!(cs.next(), Some(c) if c.is_ascii_lowercase())
        && cs.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn valid_version(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

pub fn valid_name(s: &str) -> bool {
    valid_pkg_name(s)
}

fn read_manifest(dir: &Path) -> Result<(String, toml::Doc), String> {
    let path = dir.join("rig.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("không đọc được {}: {e}", path.display()))?
        .replace("\r\n", "\n");
    let doc = toml::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok((text, doc))
}

/// `rig.toml` đã phân tích (chỉ các khoá hợp lệ).
#[derive(Clone, Debug)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    /// (tên phụ thuộc, đường dẫn đã khai báo)
    pub deps: Vec<(String, String)>,
    /// nội dung đã chuẩn hoá LF
    pub text: String,
}

/// Phân tích `rig.toml` (chỉ các khoá hợp lệ).
pub fn parse_manifest(dir: &Path) -> Result<Manifest, String> {
    let (text, doc) = read_manifest(dir)?;
    let at = |m: &str| format!("{}: {m}", dir.join("rig.toml").display());
    for sec in doc.keys() {
        if sec != "package" && sec != "dependencies" {
            return Err(at(&format!("mục `[{sec}]` không hỗ trợ")));
        }
    }
    let Some(pkg) = doc.get("package") else {
        return Err(at("thiếu mục [package]"));
    };
    for (k, (_, line)) in pkg {
        if k != "name" && k != "version" {
            return Err(at(&format!("dòng {line}: khoá package.`{k}` không hỗ trợ")));
        }
    }
    let get = |k: &str| match pkg.get(k) {
        Some((Value::Str(s), _)) => Ok(s.clone()),
        Some((_, line)) => Err(at(&format!("dòng {line}: `{k}` phải là chuỗi"))),
        None => Err(at(&format!("thiếu package.{k}"))),
    };
    let name = get("name")?;
    let version = get("version")?;
    if !valid_pkg_name(&name) {
        return Err(at(&format!(
            "tên gói `{name}` không hợp lệ (chữ thường/số/`_`, bắt đầu bằng chữ cái)"
        )));
    }
    if !valid_version(&version) {
        return Err(at(&format!(
            "phiên bản `{version}` phải dạng MAJOR.MINOR.PATCH"
        )));
    }
    let mut deps = Vec::new();
    if let Some(ds) = doc.get("dependencies") {
        for (k, (v, line)) in ds {
            if !valid_pkg_name(k) {
                return Err(at(&format!(
                    "dòng {line}: tên phụ thuộc `{k}` không hợp lệ"
                )));
            }
            let Value::Table(t) = v else {
                return Err(at(&format!(
                    "dòng {line}: phụ thuộc `{k}` phải là {{ path = \"…\" }} (chưa có registry)"
                )));
            };
            for key in t.keys() {
                if key != "path" {
                    return Err(at(&format!(
                        "dòng {line}: khoá `{key}` không hỗ trợ (chỉ `path`; chưa có registry)"
                    )));
                }
            }
            let Some(p) = t.get("path") else {
                return Err(at(&format!("dòng {line}: phụ thuộc `{k}` thiếu `path`")));
            };
            deps.push((k.clone(), p.clone()));
        }
    }
    Ok(Manifest {
        name,
        version,
        deps,
        text,
    })
}

fn read_sources(dir: &Path) -> Result<Vec<(String, String)>, String> {
    let src = dir.join("src");
    let rd = std::fs::read_dir(&src)
        .map_err(|e| format!("không đọc được thư mục {}: {e}", src.display()))?;
    let mut files = Vec::new();
    for e in rd {
        let e = e.map_err(|e| format!("lỗi đọc {}: {e}", src.display()))?;
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if p.is_dir() {
            return Err(format!(
                "{}: thư mục con `{name}` chưa hỗ trợ (mỗi tệp .gt là một module phẳng)",
                src.display()
            ));
        }
        if !name.ends_with(".gt") {
            continue;
        }
        let text = std::fs::read_to_string(&p)
            .map_err(|e| format!("không đọc được {}: {e}", p.display()))?
            .replace("\r\n", "\n");
        files.push((name, text));
    }
    files.sort();
    if files.is_empty() {
        return Err(format!("{} không có tệp .gt nào", src.display()));
    }
    Ok(files)
}

struct Loader {
    /// đường dẫn chuẩn hoá → chỉ số trong `pkgs`
    seen: BTreeMap<PathBuf, usize>,
    pkgs: Vec<Package>,
    stack: Vec<PathBuf>,
}

impl Loader {
    fn load(
        &mut self,
        dir: &Path,
        expect_name: Option<&str>,
        is_root: bool,
    ) -> Result<usize, String> {
        let canon = dir
            .canonicalize()
            .map_err(|e| format!("không tìm thấy gói tại {}: {e}", dir.display()))?;
        if let Some(i) = self.seen.get(&canon) {
            return Ok(*i);
        }
        if self.stack.contains(&canon) {
            let chain: Vec<String> = self
                .stack
                .iter()
                .map(|p| p.display().to_string())
                .chain(std::iter::once(canon.display().to_string()))
                .collect();
            return Err(format!("phụ thuộc vòng: {}", chain.join(" → ")));
        }
        let Manifest {
            name,
            version,
            deps,
            text: manifest_text,
        } = parse_manifest(&canon)?;
        if let Some(e) = expect_name
            && e != name
        {
            return Err(format!(
                "{}: tên gói là `{name}` nhưng được khai là phụ thuộc `{e}`",
                canon.display()
            ));
        }
        if let Some(other) = self.pkgs.iter().find(|p| p.name == name) {
            return Err(format!(
                "hai gói khác nhau cùng tên `{name}`: {} và {}",
                other.dir.display(),
                canon.display()
            ));
        }
        self.stack.push(canon.clone());
        for (dn, dp) in &deps {
            self.load(&canon.join(dp), Some(dn), false)?;
        }
        self.stack.pop();
        let files = read_sources(&canon)?;
        let idx = self.pkgs.len();
        self.pkgs.push(Package {
            name,
            version,
            dir: canon.clone(),
            deps,
            files,
            manifest_text,
            is_root,
        });
        self.seen.insert(canon, idx);
        Ok(idx)
    }
}

/// Đọc cả đồ thị từ thư mục gói gốc.
pub fn load(root_dir: &Path) -> Result<Project, String> {
    let mut l = Loader {
        seen: BTreeMap::new(),
        pkgs: Vec::new(),
        stack: Vec::new(),
    };
    l.load(root_dir, None, true)?;
    let p = Project { pkgs: l.pkgs };
    check_modules(&p)?;
    Ok(p)
}

/// Quy tắc module: (1) tên tệp không va chạm giữa các gói; (2) tệp của gói PHỤ THUỘC phải tên `<gói>.gt` hoặc `<gói>_….gt`
/// (đặt tiền tố để module của các gói không bao giờ đè nhau).
fn check_modules(p: &Project) -> Result<(), String> {
    let mut owner: BTreeMap<&str, &str> = BTreeMap::new();
    for pkg in &p.pkgs {
        for (f, _) in &pkg.files {
            let stem = f.strip_suffix(".gt").unwrap_or(f);
            if !pkg.is_root && stem != pkg.name && !stem.starts_with(&format!("{}_", pkg.name)) {
                return Err(format!(
                    "gói `{}`: tệp `{f}` phải tên `{0}.gt` hoặc `{0}_….gt` (tiền tố tên gói, để module của các gói không va chạm)",
                    pkg.name
                ));
            }
            if let Some(o) = owner.insert(f, &pkg.name) {
                return Err(format!(
                    "module `{stem}` có ở cả gói `{o}` và gói `{}`",
                    pkg.name
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_versions() {
        assert!(valid_pkg_name("a1_b"));
        for bad in ["", "A", "1a", "a-b", "a b", "é"] {
            assert!(!valid_pkg_name(bad), "{bad}");
        }
        assert!(valid_version("0.1.0") && valid_version("10.20.30"));
        for bad in ["1.0", "1.0.0.0", "a.b.c", "1..0", "1.0.0-x", ""] {
            assert!(!valid_version(bad), "{bad}");
        }
    }
}
