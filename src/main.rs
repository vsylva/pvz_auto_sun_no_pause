use std::fs::{self, OpenOptions};
use std::io::{Read, Write};

type AnyResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Replacement {
    Keep,
    Value(u8),
}

struct Patch<'a> {
    name: &'a str,
    sig: &'a str,
    rep: &'a str,
}

fn main() -> AnyResult<()> {
    let file_path = "PlantsVsZombies.exe";

    let mut data = Vec::new();
    OpenOptions::new()
        .read(true)
        .open(file_path)
        .map_err(|_| "找不到文件: ".to_string() + file_path)?
        .read_to_end(&mut data)
        .map_err(|e| format!("读取失败: {e}"))?;

    println!("成功读取文件，大小: {} 字节", data.len());

    let patches = [
        Patch {
            name: "第一个特征码",
            sig: "75 09 8B FB E8 75 F5 FF",
            rep: "EB ?? ?? ?? ?? ?? ?? ??",
        },
        Patch {
            name: "第二个特征码",
            sig: "55 8B EC 83 E4 F8 64 A1 00 00 00 00 6A FF 68 B8 D3 6E 00 50",
            rep: "C2 04 00 ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ??",
        },
    ];

    let mut result = true;
    for p in patches.iter() {
        match apply_patch(p, &mut data) {
            Ok(off) => println!("{}已替换，偏移位置: 0x{:X}", p.name, off),
            Err(e) => {
                result = false;
                eprintln!("{}失败：{e}", p.name)
            }
        }
    }

    OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_path)?
        .write_all(&data)?;

    if result {
        let file_bak_path = format!("{file_path}.bak");

        println!("文件修改完成！");

        fs::write(&file_bak_path, &data).map_err(|e| format!("写入失败: {e}"))?;

        println!("成功备份原exe为: {file_bak_path}");
    }

    press_enter_to_continue();
    Ok(())
}

fn press_enter_to_continue() {
    print!("\nPress ENTER to continue...");
    let _ = ::std::io::Write::flush(&mut ::std::io::stdout());
    let _ = ::std::io::stdin().read_line(&mut String::new());
}

fn parse_sig<S: AsRef<str>>(sig: S) -> AnyResult<Vec<Option<u8>>> {
    let mut out = Vec::new();
    for tok in sig.as_ref().split_whitespace() {
        if matches!(tok, "?" | "??" | "*" | "**") {
            out.push(None);
        } else {
            let v = u8::from_str_radix(tok, 16)?;
            out.push(Some(v));
        }
    }
    Ok(out)
}

fn parse_rep<S: AsRef<str>>(rep: S) -> AnyResult<Vec<Replacement>> {
    let mut out = Vec::new();
    for tok in rep.as_ref().split_whitespace() {
        if matches!(tok, "?" | "??" | "*" | "**") {
            out.push(Replacement::Keep);
        } else {
            let v = u8::from_str_radix(tok, 16)?;
            out.push(Replacement::Value(v));
        }
    }
    Ok(out)
}

fn sig_find<S: AsRef<str>, U: AsRef<[u8]>>(sig: S, data: U) -> AnyResult<usize> {
    let pat = parse_sig(sig)?;
    let data = data.as_ref();
    let n = data.len();
    let m = pat.len();
    if m == 0 || m > n {
        return Err("\"pat\" not found".into());
    }

    let mut skip = vec![m; 256];
    for i in 0..m - 1 {
        if let Some(b) = pat[i] {
            skip[b as usize] = (m - 1) - i;
        } else {
        }
    }

    let mut i = m - 1;
    while i < n {
        let mut j = m - 1;
        let mut k = i;
        while j < m {
            match pat[j] {
                Some(b) => {
                    if data[k] != b {
                        break;
                    }
                }
                None => {}
            }
            if j == 0 {
                return Ok(k);
            }
            j -= 1;
            k -= 1;
        }
        let step = skip[data[i] as usize];
        i = i.saturating_add(step.max(1));
    }

    Err("\"pat\" not found".into())
}

fn apply_patch(patch: &Patch, data: &mut [u8]) -> AnyResult<usize> {
    let off = sig_find(patch.sig, &data)?;
    let pat = parse_sig(patch.sig)?;
    let rep = parse_rep(patch.rep)?;
    if pat.len() != rep.len() {
        return Err(format!(
            "替换模板长度与特征码不一致: pat={} rep={}",
            pat.len(),
            rep.len()
        )
        .into());
    }
    let end = off + pat.len();
    if end > data.len() {
        return Err("替换超出文件范围".into());
    }

    for (idx, r) in rep.iter().enumerate() {
        match r {
            Replacement::Keep => {}
            Replacement::Value(v) => {
                data[off + idx] = *v;
            }
        }
    }
    Ok(off)
}
